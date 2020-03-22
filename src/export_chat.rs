// use crate::dc_tools::*;
use crate::chat::*;
use crate::constants::DC_CONTACT_ID_SELF;
use crate::contact::*;
use crate::context::Context;
use crate::error::Error;
use crate::message::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ExportChatResult {
    html: String,
    referenced_blobs: Vec<String>,
}

struct ContactInfo {
    name: String,
    initial: String,
    color: String,
    profile_img: Option<String>,
}

// pub fn packExportedChat(artifact:ExportChatResult) -> ? {}

pub fn export_chat(context: &Context, chat_id: ChatId) -> ExportChatResult {
    let mut blobs = Vec::new();
    let mut chat_author_ids = Vec::new();
    // get all messages
    let messages: Vec<std::result::Result<Message, Error>> =
        get_chat_msgs(context, chat_id, 0, None)
            .into_iter()
            .map(|msg_id| Message::load_from_db(context, msg_id))
            .collect();
    // push all referenced blobs and populate contactid list
    for message in &messages {
        if let Ok(msg) = &message {
            let filename = msg.get_filename();
            if let Some(file) = filename {
                // push referenced blobs (attachments)
                blobs.push(file);
            }
            chat_author_ids.push(msg.from_id);
        }
    }
    // deduplicate contact list and load the contacts
    chat_author_ids.dedup();
    // chache information about the authors
    let mut chat_authors: HashMap<u32, ContactInfo> = HashMap::new();
    chat_authors.insert(0, ContactInfo {
        name: "Err: Contact not found".to_owned(),
        initial: "#".to_owned(),
        profile_img: None,
        color: "grey".to_owned(),
    });
    for author_id in chat_author_ids {
        let contact = Contact::get_by_id(context, author_id);
        if let Ok(c) = contact {
            let profile_img_path: String;
            if let Some(path) = c.get_profile_image(context) {
                profile_img_path = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("")).to_str().unwrap().to_owned();
                 // push referenced blobs (avatars)
                 blobs.push(profile_img_path.clone());
            } else {
                profile_img_path = "".to_owned();
            }
            chat_authors.insert(
                author_id,
                ContactInfo {
                    name: c.get_display_name().to_owned(),
                    initial: "#".to_owned(), // TODO
                    profile_img: match profile_img_path != "" {
                        true => Some(profile_img_path),
                        false => None,
                    },
                    color: "rgb(18, 126, 208)".to_owned(), // TODO
                },
            );
        }
    }

    // run message_to_html for each message and generate the html that way
    let mut html_messages:Vec<String> = Vec::new();
    for message in messages {
        if let Ok(msg) = message {
            html_messages.push(message_to_html(&chat_authors, msg));
        } else {
            html_messages.push(
                format!(
                    r#"<li>
                        <div class='message error'>
                            <div class="msg-container">
                                <div class="msg-body">
                                    <div dir="auto" class="text">{:?}</div>
                                </div>
                            </div>
                        </div>
                    </li>"#,
                    message.unwrap_err()
                )
            );
        }
    }

    // todo chat image, chat name and so on..

    // todo option to export locations as kml?

    // todo export message infos and save them to txt files
    // (those can be linked from the messages, they are stored in msg_info/[msg-id].txt)

    ExportChatResult {
        html: format!(r#"<ul>{}</ul>"#, html_messages.join("")),
        referenced_blobs: blobs,
    }
}

fn message_to_html(author_cache: &HashMap<u32, ContactInfo>, message: Message) -> String {
    let author: &ContactInfo = {
        if let Some(c) = author_cache.get(&message.get_from_id()) {
            c
        } else {
            author_cache.get(&0).unwrap()
        }
    };

    let avatar: String = {
        if let Some(profile_img) = &author.profile_img {
            format!(
                "<div class=\"author-avatar\">\
                    <img\
                        alt=\"{author_name}\"\
                        src=\"blobs/{author_avatar_src}\"\
                    />\
                </div>",
                author_name = author.name,
                author_avatar_src = profile_img
            )
        } else {
            format!(
                "<div class=\"author-avatar default\" alt=\"{name}\">\
                <div class=\"label\" style=\"background-color: {color}\">\
                    {initial}\
                </div>\
            </div>",
                name = author.name,
                initial = author.initial,
                color = author.color
            )
        }
    };

    // save and refernce message source code somehow?

    //todo support images / voice message / attachments

    format!(
        "<li>\
            <div class=\"message {direction}\">\
                {avatar}\
                <div class=\"msg-container\">\
                    <span class=\"author\" style=\"color: {author_color};\">{author_name}</span>\
                    <div class=\"msg-body\">\
                        <div dir=\"auto\" class=\"text\">\
                        {content}\
                        </div>\
                        <div class=\"metadata\">\
                            {encryption}\
                            <span class=\"date date--{direction}\" title=\"{full_time}\">{relative_time}</span>\
                            <span class=\"spacer\"></span>\
                        </div>\
                    </div>\
                </div>\
            <div>\
        </li>",
        direction = match message.from_id == DC_CONTACT_ID_SELF {
            true => "outgoing",
            false => "incomming",
        },
        avatar = avatar,
        author_name = author.name,
        author_color = author.color,
        content = message.get_text().unwrap_or_else(|| "".to_owned()),
        encryption = match message.get_showpadlock() {
            true => r#"<div aria-label="Encryption padlock" class="padlock-icon"></div>"#,
            false => "",
        },
        full_time = "Tue, Feb 25, 2020 3:49 PM", // message.get_timestamp() ?
        relative_time = "Tue 3:49 PM"
    )

    // todo link to raw message data / link to message info
}

//TODO tests