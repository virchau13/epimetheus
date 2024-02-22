use std::time::{Duration, Instant};

use async_trait::async_trait;
use serenity::all::{GatewayIntents, Message};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::client::{Context, EventHandler};
use serenity::Client;

mod dice;
mod os;
mod util;

struct Handler {
    start_time: Instant,
}

impl Handler {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    fn uptime(&self) -> String {
        let mut time_elapsed = Instant::now().duration_since(self.start_time).as_secs();
        let secs = time_elapsed % 60;
        time_elapsed /= 60;
        let mins = time_elapsed % 60;
        time_elapsed /= 60;
        let hours = time_elapsed;
        format!("{hours}h{mins}m{secs}s")
    }

    async fn proc_msg(&self, ctx: Context, msg: Message) -> anyhow::Result<()> {
        let content = &msg.content;
        if let Some(first_word) = content.split_whitespace().next() {
            match first_word {
                "%roll" | "%eval" | "%evaluate" | "%calc" | "%calculate" => {
                    if let Some((_, expr)) = content.split_once(' ') {
                        let mut expr = expr.trim();
                        if expr.starts_with("```") && expr.ends_with("```") {
                            expr = &expr[3..expr.len() - 3];
                        } else if expr.starts_with("`") && expr.ends_with("`") {
                            expr = &expr[1..expr.len() - 1];
                        }
                        match tokio::time::timeout(Duration::from_millis(50), dice::eval(expr))
                            .await
                        {
                            Ok(evalres) => match evalres {
                                Ok(v) => {
                                    let mut s = String::new();
                                    v.display(&mut s).await;
                                    msg.channel_id.say(&ctx, s).await?;
                                }
                                Err(e) => {
                                    msg.channel_id
                                        .say(&ctx, format!("evaluation error: {e}"))
                                        .await?;
                                }
                            },
                            Err(_) => {
                                msg.channel_id.say(&ctx, format!("evaluation exceeded max duration of 50 milliseconds, execution halted")).await?;
                            }
                        }
                    }
                }
                "%ping" => {
                    msg.channel_id.say(&ctx, format!("pong.")).await?;
                }
                "%checkhealth" => {
                    let mut embed = CreateEmbed::new()
                        .title("Alive and healthy")
                        .color(0x00ff00)
                        .field("Uptime", self.uptime(), true);
                    match os::get_mem_usage().await {
                        Ok((virt, rss)) => {
                            embed = embed
                                .field("Virtual memory", os::fmt_bibytes(virt), true)
                                .field("RSS", os::fmt_bibytes(rss), true);
                        }
                        Err(_) => todo!(),
                    }
                    let builder = CreateMessage::new().embed(embed);
                    msg.channel_id.send_message(&ctx, builder).await?;
                }
                "%help" => {}
                _ => {}
            }
        }
        Ok(())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let channel = msg.channel_id;
        if msg.content.starts_with('%') {
            if let Err(e) = self.proc_msg(ctx.clone(), msg).await {
                let _ = channel.say(&ctx, format!("error processing command: {:?}", e)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("you forgot your token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler::new())
        .await
        .expect("error creating client");

    client.start().await.unwrap();
}
