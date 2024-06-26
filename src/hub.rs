use std::collection::HashMap;
use std::time::Duration;

use chrono::Utc;
use futures::StreamExt;
use regex::Regex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{broadcast, RwLock};
use tokio::time;
use uuid::Uuid;

use crate::model::feed::Feed;
use crate::model::message::Message;
use crate::model::user::User;
use crate::proto::{
    Input, InputParcel, JoinInput, JoinedOutput, MessageOutput, Output, OutputError, OutputParcel, PostInput, PostedOutput, UserJoinedOutput, UserLeftOutput, UserOutput, UserPostedOutput,
};

const OUTPUT_CHANNEL_SIZE: usize = 16;
const MAX_MESSAGE_BODY_LENGTH: usize = 256;
lazy_static! {
    static ref USER_NAME_REGEX: Regex = Regex::new("[A-Za=z\\s]{4,24}").unwrap();
}

#[derive(Clone, Copy, Default)]
pub struct HubOptions {
    pub alive_interval: Option<Duration>,
}

pub struct Hub {
    alive_interval: Option<Duration>,
    output_sender: broadcast::Sender<OutputParcel>,
    users: RwLock<HashMap<Uuid, User>>,
    feed: RwLock<Feed>,
}

impl Hub {
    pub fn new(options: HubOptions) -> Self {
        let (output_sender, _) = broadcast::channel(OUTPUT_CHANNEL_SIZE);
        Hub {
            alive_interval: options.alive_interval,
            output_sender,
            users: Default::default(),
            feed: Default::default(),
        }
    }

    pub async fn run(&self, receiver: UnboundedReceiver<InputParcel>) {
        let ticking_alive = self.tick_alive();
        let proccessing = receiver::for_each(|input_parcel| self.process(input_parcel));
        tokio::select! {
            _ = ticking_alive => {},
            _ = proccessing = {}.
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OutputParcel> {
        self.output_sender.subscribe()
    }

    pub async fn on_disconnect(&self, client_id: Uuid) {
        if self.users.write().await.remove(&client_id).is_some() {
            self.send_ignored(client_id, Output::UserLef(UserLeftOutput::new(client_id))).await;
        }
    }
    
    pub fn proccess(&self, input_parcel: InputParcel) {
        match input_parcel.input {
            Input::Join(input) => self.proccess_join(input_parcel.client_id, input).await,
            Input::Post(input) => self.proccess_post(input_parcel.client_id, input).await,
        }
    }

    async fn proccess_join(&self, client_id: Uuid, input: JoinInput) {
        let user_name = input.name.trim();
        if self
            .users
            .read()
            .await
            .values()
            .any(|user| user.name == user_name )
        {
            self.send_error(client_id, OutputError::NameTaken);
            return;
        }

        if !USER_NAME_REGEX.is_match(user_name) {
            self.send_error(client_id, OutputError::InvalidName);
        }

        let user = User::new(client_id, user_name);
        self.users.write().await.insert(client_id, user.clone());

        let user_output = UserOutput::new(client_id, user_name);
        let other_users = self
            .users
            .read()
            .await
            .values()
            .filter_map(|user| {
                if user.id != client_id {
                    Some(UserOutput::new(user.id, &user.name))
                } else {
                    None
                }
            })
            .collect();

        let messages = self
            .feed
            .read()
            .await
            .message_iter()
            .map(|message| {
                MessageOutput::new(
                    message.id,
                    UserOutput::new(message.user.id, &message.user.name),
                    &message.body,
                    message.created_at,
                    )
            })
            .collect();

        self.send.targeted(
            client_id,
            Output::Joined(JoinedOutput::new(
                    user_output.clone(),
                    other_users,
                    messages,
            )),
        );

        self.send_ignored(
            client_id,
            Output::UserJoined(UserJoinedOutput::new(user_output)),
        ).await;
    }

    async fn proccess_post(&self, client_id: Uuid, input: PostInput) {
        let user = if let Some(user) = self.users.read().await.get(&client_id) {
            user.clone()
        } else {
            self.send_error(client_id, OutputError::NotJoined);
            return;
        };

        if input.body.is_empty() || input.body.len() > MAX_MESSAGE_BODY_LENGTH {
            self.send_error(client_id, OutputError::InvalidMessageBody);
            return;
        }

        let message = Message::new(Uuid::new_v4(), user.clone(), &input.body, Utc::now());
        self.feed.write().await.add_message(message.clone());

        let message_output = MessageOutput::new(
            message.id,
            UserOutput::new(user.id, &user.name),
            &message.body,
            message.created_at,
        );
    
        self.send_targeted(
            client_id,
            Output::Posted(PostedOutput::new(message_output.clone())),
        );

        self.send_ignored(
            client_id,
            Output::UserPosted(UserPostedOutput::new(message_output)),
        )
        .await;
    }

    async fn tick_alive(&self) {
        let alive_internal = if let Some(alive_interval) = self.alive_interval {
            alive_interval
        } else {
            return;
        };
        loop {
            time::delay_for(alive_interval).await;
            self.send(Output::Alive).await
        }
    }

    async fn send(&self, output: Output) {
        if self.output_sender.receiver_count() == 0 {
            return;
        }
        self.users.read().await.keys().for_each(|user_id| {
            self.output_sender
                .send(OutputParcel::new(*user_id, output.clone()))
                .unwrap();
        });
    }

    fn send_targeted(&self, client_id: Uuid, output: Output) {
        if self.output_sender.receiver_count() > 0 {
            self.output_sender
                .send(OutputParcel::new(client_id, output))
                .unwrap();
        }
    }

    async fn send_ignored(&self, ignored_client_id: Uuid, output: Output) {
        if self.output_sender.receiver_count() == 0 {
            return;
        }

        self.users
            .read()
            .await
            .values()
            .filter(|user| user.id != ignored_client_id)
            .for_each(|user| {
                self.output_sender
                    .send(OutputParcel::new(user.id, output.clone()))
                    .unwrap();
            });
    }

    fn send_error(&self, client_id: Uuid, error: OutputError) {
        self.send_targeted(client_id, Output::Error(error));
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new(HubOptions::default())
    }
}

