use std::time::{Duration, Instant};

use async_trait::async_trait;
use serenity::all::{GatewayIntents, Message, MessageReference};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::client::{Context, EventHandler};
use serenity::Client;

mod dice;
mod os;
mod util;

struct Handler {
    start_time: Instant,
}

#[cfg(not(debug_assertions))]
const PREFIX: &str = "%";

#[cfg(debug_assertions)]
const PREFIX: &str = "t%";

async fn reply(ctx: &Context, msg: Message, builder: CreateMessage) -> serenity::Result<()> {
    let msgref = MessageReference::from((msg.channel_id, msg.id));
    msg.channel_id.send_message(&ctx, builder.reference_message(msgref)).await?;
    Ok(())
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
        let mut words = content.split_whitespace();
        if let Some(first_word) = words.next() {
            match &first_word[PREFIX.len()..] {
                "roll" | "eval" | "evaluate" | "calc" | "calculate" => {
                    if let Some((_, expr)) = content.split_once(' ') {
                        let mut expr = expr.trim();
                        if expr.starts_with("```") && expr.ends_with("```") {
                            expr = &expr[3..expr.len() - 3];
                        } else if expr.starts_with('`') && expr.ends_with('`') {
                            expr = &expr[1..expr.len() - 1];
                        }
                        match tokio::time::timeout(Duration::from_millis(50), dice::eval(expr))
                            .await
                        {
                            Ok(evalres) => match evalres {
                                Ok(v) => {
                                    let mut s = String::new();
                                    v.display(&mut s).await;
                                    reply(&ctx, msg, CreateMessage::new().content(s)).await?;
                                }
                                Err(e) => {
                                    reply(&ctx, msg, CreateMessage::new().content(format!("evaluation error: {e}"))).await?;
                                }
                            },
                            Err(_) => {
                                reply(&ctx, msg, CreateMessage::new().content("evaluation exceeded max duration of 50 milliseconds, execution halted")).await?;
                            }
                        }
                    }
                },
                "ping" => {
                    reply(&ctx, msg, CreateMessage::new().content("pong.")).await?;
                }
                "checkhealth" => {
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
                    reply(&ctx, msg, builder).await?;
                },
                "help" => {
                    match words.next() {
                        Some("roll") => {
                            let op_list = dice::get_op_string_list();
                            let embed = CreateEmbed::new()
                                .title("`%roll`*`expression`*")
                                .color(0xA526B3)
                                .description(indoc::indoc! {r#"
                                    Synonyms: **`%eval`, `%calc`**
                                    Evaluate the expression given.
                                    **Common examples**
                                    `%roll 4d6+7`: Roll 4 6-sided dice, add them all up, and add 7 to that.
                                    `%roll 2d20H1`: Roll 2 20-sided dice and choose the **h**ighest one. (Advantage. For disadvantage, use `L`.)
                                    `%roll d2!`: Roll an exploding d2.
                                    `%roll d10!(9,10)!`: Roll a d10 that explodes on outcomes of either 9 or 10.
                                    `%roll d(1,4,5)`: Roll a dice with 3 custom sides: one side with 1, one side with 4, and one side with 5.
                                "#})
                                .field("Regular operators", op_list, false);
                            let builder = CreateMessage::new().embed(embed);
                            // TODO pages
                            reply(&ctx, msg, builder).await?;
                        },
                        Some("ping") => {
                            let embed = CreateEmbed::new()
                                .title("`%ping`")
                                .color(0xA526B3)
                                .description(concat!(
                                    "Makes the bot respond `pong.`\n\n",
                                    "||",
                                    "You're a curious person, huh? That's awesome. Curiosity is what makes the world go round. ",
                                    "You're a good person. You're doing a good job. There are more people who love you than you know.",
                                    "||"
                                ));
                            let builder = CreateMessage::new().embed(embed);
                            reply(&ctx, msg, builder).await?;
                        },
                        Some("checkhealth") => {
                            let embed = CreateEmbed::new()
                                .title("`%checkhealth`")
                                .color(0xA526B3)
                                .description(concat!(
                                    "Reports the health of the bot and its server.\n",
                                    "So far this only includes the uptime, virtual memory usage, and resident memory usage, ",
                                    "but feel free to suggest any other diagnostics you'd want to see."
                                ));
                            let builder = CreateMessage::new().embed(embed);
                            reply(&ctx, msg, builder).await?;
                        },
                        Some("help") => {
                            if let Some("help") = words.next() {
                                reply(&ctx, msg, CreateMessage::new().content("Now you're just being silly.")).await?;
                            } else {
                                let embed = CreateEmbed::new()
                                    .title("`%help`*`[command]`*")
                                    .color(0xA526B3)
                                    .description(concat!(
                                        "The all-purpose help command.\n",
                                        "Specific variations:"
                                    ))
                                    .field("`%help`", "Print the overall help page, containing all commands.", false)
                                    .field("`%help`*`command`*", "Print the help page for a specific command.", false);
                                let builder = CreateMessage::new().embed(embed);
                                reply(&ctx, msg, builder).await?;
                            }
                        }
                        _ => {
                            let embed = CreateEmbed::new()
                                .title("All commands")
                                .color(0xA526B3)
                                .description("TL;DR: `%roll`, `%help`, `%checkhealth`, `%ping`")
                                .field("`%roll`", "Calculate dice values, with arbitrary mathematical expressions.\ne.g. `%roll 4d6+7`\n Synonyms: **`%calc`, `%eval`**", false)
                                .field("`%help`", "Display the help-page for a specific command.\ne.g. `%help roll`.", false)
                                .field("`%checkhealth`", "Display a dialog with bot health information.", true)
                                .field("`%ping`", "Make the bot respond `pong.`", true);
                            let builder = CreateMessage::new().embed(embed);
                            reply(&ctx, msg, builder).await?;
                        }
                    }
                }
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
        if msg.content.starts_with(PREFIX) {
            if let Err(e) = self.proc_msg(ctx.clone(), msg).await {
                let _ = channel.say(&ctx, format!("error processing command: {e:?}")).await;
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
