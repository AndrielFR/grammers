// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::types::{Dialog, EntitySet, IterBuffer};
use crate::ClientHandle;
use grammers_mtsender::InvocationError;
use grammers_tl_types as tl;

const MAX_LIMIT: usize = 100;

pub type DialogIter = IterBuffer<tl::functions::messages::GetDialogs, Dialog>;

impl DialogIter {
    fn new(client: &ClientHandle) -> Self {
        // TODO let users tweak all the options from the request
        Self::from_request(
            client,
            MAX_LIMIT,
            tl::functions::messages::GetDialogs {
                exclude_pinned: false,
                folder_id: None,
                offset_date: 0,
                offset_id: 0,
                offset_peer: tl::enums::InputPeer::Empty,
                limit: 0,
                hash: 0,
            },
        )
    }

    /// Determines how many dialogs there are in total.
    ///
    /// This only performs a network call if `next` has not been called before.
    pub async fn total(&mut self) -> Result<usize, InvocationError> {
        if let Some(total) = self.total {
            return Ok(total);
        }

        use tl::enums::messages::Dialogs;

        self.request.limit = 1;
        let total = match self.client.invoke(&self.request).await? {
            Dialogs::Dialogs(dialogs) => dialogs.dialogs.len(),
            Dialogs::Slice(dialogs) => dialogs.count as usize,
            Dialogs::NotModified(dialogs) => dialogs.count as usize,
        };
        self.total = Some(total);
        Ok(total)
    }

    /// Return the next `Dialog` from the internal buffer, filling the buffer previously if it's
    /// empty.
    ///
    /// Returns `None` if the `limit` is reached or there are no dialogs left.
    pub async fn next(&mut self) -> Result<Option<Dialog>, InvocationError> {
        if let Some(result) = self.next_raw() {
            return result;
        }

        use tl::enums::messages::Dialogs;

        self.request.limit = self.determine_limit(MAX_LIMIT);
        let (dialogs, messages, users, chats) = match self.client.invoke(&self.request).await? {
            Dialogs::Dialogs(d) => {
                self.last_chunk = true;
                self.total = Some(d.dialogs.len());
                (d.dialogs, d.messages, d.users, d.chats)
            }
            Dialogs::Slice(d) => {
                self.last_chunk = d.dialogs.len() < self.request.limit as usize;
                self.total = Some(d.count as usize);
                (d.dialogs, d.messages, d.users, d.chats)
            }
            Dialogs::NotModified(_) => {
                panic!("API returned Dialogs::NotModified even though hash = 0")
            }
        };

        let entities = EntitySet::new(users, chats);
        // TODO MessageSet

        self.buffer.extend(
            dialogs
                .into_iter()
                .map(|dialog| Dialog::new(dialog, &messages, &entities)),
        );

        // Don't bother updating offsets if this is the last time stuff has to be fetched.
        if !self.last_chunk && !self.buffer.is_empty() {
            self.request.exclude_pinned = true;
            if let Some(last_message) = self
                .buffer
                .iter()
                .rev()
                .find_map(|dialog| dialog.last_message.as_ref())
            {
                // TODO build some abstractions to extract common fields
                match last_message {
                    tl::enums::Message::Message(message) => {
                        self.request.offset_date = message.date;
                        self.request.offset_id = message.id;
                    }
                    tl::enums::Message::Service(message) => {
                        self.request.offset_date = message.date;
                        self.request.offset_id = message.id;
                    }
                    tl::enums::Message::Empty(message) => {
                        self.request.offset_id = message.id;
                    }
                }
            }
            self.request.offset_peer = self.buffer[self.buffer.len() - 1].input_peer();
        }

        Ok(self.pop_item())
    }
}

impl ClientHandle {
    /// Returns a new iterator over the dialogs.
    pub fn iter_dialogs(&self) -> DialogIter {
        DialogIter::new(self)
    }

    /// Deletes a dialog, effectively removing it from your list of open conversations.
    ///
    /// The dialog is only deleted for yourself.
    ///
    /// Deleting a dialog effectively clears the message history and "kicks" you from it.
    ///
    /// For groups and channels, this is the same as leaving said chat. This method does **not**
    /// delete the chat itself (the chat still exists and the other members will remain inside).
    pub async fn delete_dialog(
        &mut self,
        chat: &tl::enums::InputPeer,
    ) -> Result<(), InvocationError> {
        use tl::enums::InputPeer::*;

        match chat {
            Empty => Ok(()),
            PeerSelf | User(_) | UserFromMessage(_) => {
                // TODO only do this if we're not a bot
                self.invoke(&tl::functions::messages::DeleteHistory {
                    just_clear: false,
                    revoke: false,
                    peer: chat.clone(),
                    max_id: 0,
                })
                .await
                .map(drop)
            }
            Chat(chat) => {
                // TODO handle PEER_ID_INVALID and ignore it (happens when trying to delete deactivated chats)
                self.invoke(&tl::functions::messages::DeleteChatUser {
                    chat_id: chat.chat_id,
                    user_id: tl::enums::InputUser::UserSelf,
                })
                .await
                .map(drop)
            }
            Channel(channel) => self
                .invoke(&tl::functions::channels::LeaveChannel {
                    channel: tl::types::InputChannel {
                        channel_id: channel.channel_id,
                        access_hash: channel.access_hash,
                    }
                    .into(),
                })
                .await
                .map(drop),
            ChannelFromMessage(channel) => self
                .invoke(&tl::functions::channels::LeaveChannel {
                    channel: tl::types::InputChannelFromMessage {
                        peer: channel.peer.clone(),
                        msg_id: channel.msg_id,
                        channel_id: channel.channel_id,
                    }
                    .into(),
                })
                .await
                .map(drop),
        }
    }
}
