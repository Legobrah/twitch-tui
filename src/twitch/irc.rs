use crate::app::AppEvent;
use crate::twitch::ChatMessage;
use tokio::sync::mpsc;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;

pub type IrcClient = TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>;

fn spawn_reader(mut incoming: mpsc::UnboundedReceiver<ServerMessage>, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        while let Some(msg) = incoming.recv().await {
            if let ServerMessage::Privmsg(pm) = msg {
                let chat_msg = ChatMessage {
                    sender: pm.sender.name,
                    message: pm.message_text,
                    system: false,
                };
                if tx.send(AppEvent::ChatMessage(chat_msg)).is_err() {
                    break;
                }
            }
        }
    });
}

pub fn connect_anonymous(
    channel: &str,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> Result<IrcClient, String> {
    let config = ClientConfig::default();
    let (incoming, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    spawn_reader(incoming, tx);

    let ch = channel.to_string();
    client
        .join(ch)
        .map_err(|e| format!("Failed to join channel: {}", e))?;

    Ok(client)
}

pub fn connect_authenticated(
    username: &str,
    token: &str,
    channel: &str,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> Result<IrcClient, String> {
    let login = StaticLoginCredentials::new(username.to_string(), Some(token.to_string()));
    let config = ClientConfig::new_simple(login);
    let (incoming, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    spawn_reader(incoming, tx);

    let ch = channel.to_string();
    client
        .join(ch)
        .map_err(|e| format!("Failed to join channel: {}", e))?;

    Ok(client)
}
