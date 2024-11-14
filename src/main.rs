mod db;
mod models;

use crate::db::establish_connection;

use std::{collections::HashSet, error::Error, fs, time::Duration};

use dotenv::dotenv;
use opml::{Outline, OPML};
use rand::prelude::SliceRandom;
use reqwest;
use rss::Channel;
use teloxide::{prelude::*, types::ParseMode};
use tokio::time::sleep;

use chatgpt::{prelude::ChatGPT, types::CompletionResponse};
use sqlx;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let bot_token = std::env::var("TG_BOT_TOKEN")
        .expect("MAILCOACH_API_TOKEN must be set.")
        .to_string();
    let channel_id: String = std::env::var("TG_CHAT_ID")
        .expect("MAILCOACH_API_TOKEN must be set.")
        .to_string();
    let bot = Bot::new(bot_token);

    // Path to the OPML file containing RSS feed URLs
    let opml_file_path = "feeds.opml";
    let rss_urls =
        load_rss_urls_from_opml(opml_file_path).expect("Failed to load RSS URLs from OPML");

    // Keep track of seen items to avoid reposting
    let seen_urls: HashSet<String> = HashSet::new();

    loop {
        if let Some(rss_feed_url) = rss_urls.choose(&mut rand::thread_rng()) {
            if let Err(err) = fetch_and_send_rss_updates(&bot, channel_id.clone(), rss_feed_url) //&mut seen_urls
                .await
            {
                eprintln!("Error fetching or sending updates: {:?}", err);
            }
        }

        // Wait 10 minutes before checking again
        sleep(Duration::from_secs(6)).await;
    }
}

// Main function to load RSS URLs from an OPML file
fn load_rss_urls_from_opml(opml_file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let opml_content = fs::read_to_string(opml_file_path)?;
    let opml = OPML::from_str(&opml_content)?;

    // Use the recursive function to gather all RSS URLs
    let mut rss_urls = Vec::new();
    extract_urls_from_outline(&opml.body.outlines, &mut rss_urls);

    println!("Your RSS URLs: {:#?}", rss_urls);
    Ok(rss_urls)
}

async fn chatgpt(url: String) -> Result<String, Box<dyn std::error::Error>> {
    let key = std::env::var("OPENAI")?;
    let client = ChatGPT::new(key)?;

    // Sending a message and getting the completion
    let response: CompletionResponse = client
        .send_message(format!("дай описание этой новости в пределах 400 символов: {}", url))
        .await?;

    println!("Response: {}", &response.message().content);

    Ok(response.message().content.clone())
}

// Recursive function to traverse nested outlines and collect xmlUrl attributes
fn extract_urls_from_outline(outlines: &[Outline], rss_urls: &mut Vec<String>) {
    for outline in outlines {
        if let Some(xml_url) = &outline.xml_url {
            rss_urls.push(xml_url.clone());
        }

        // If there are nested outlines, recurse into them
        if !outline.outlines.is_empty() {
            extract_urls_from_outline(&outline.outlines, rss_urls);
        }
    }
}

async fn insert_seen_post(pool: &sqlx::PgPool, link: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO seen_posts (link) VALUES ($1)")
        .bind(link)
        .execute(pool)
        .await?;
    Ok(())
}

async fn fetch_and_send_rss_updates(
    bot: &Bot,
    channel_id: String,
    rss_feed_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fetch the RSS feed
    let content = reqwest::get(rss_feed_url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    // Establish a connection to the database
    let pool = establish_connection().await?;

    for item in channel.items() {
        if let Some(item_link) = item.link() {
            // Check if the link exists in the database
            let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM seen_posts WHERE link = $1")
                .bind(item_link)
                .fetch_one(&pool)
                .await?;

            if exists.0 == 0 {
                // Insert the new seen post
                sqlx::query("INSERT INTO seen_posts (link) VALUES ($1)")
                    .bind(item_link)
                    .execute(&pool)
                    .await?;

                let link1 = item_link.to_string();
                let title = item.title().unwrap_or("No Title").to_string();
                let caption = "<a href=\"https://t.me/Tech_Chronicle/\">Tech Chronicle</a>";
                let content: Option<&str> = item.content();

                // Format the message with content if available
                // let message = if let Some(text) = content {
                //     let text = chatgpt(text.to_string()).await?;
                //     text
                // } else {
                //     "no text".to_string()
                // };
                let message = chatgpt(link1.clone()).await?;

                // Send the message to the Telegram channel
                bot.send_message(
                    channel_id.clone(),
                    format!(
                        "<b># {}</b> | <a href=\"{}\">источник</a>\n\n{}\n\n{}",
                        title, link1.clone(), message, caption,
                    ),
                )
                .parse_mode(ParseMode::Html)
                .send()
                .await?;

                // Delay for 15 minutes before sending the next message
                sleep(Duration::from_secs(5)).await;

                break; // Stop after sending 1 element from one RSS URL
            }
        }
    }

    Ok(())
}
