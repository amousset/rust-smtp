use crate::message::{
    header::{self, Header},
    IntoBody, SinglePart,
};
use mime::Mime;

#[derive(Clone, Copy)]
enum Disposition {
    Attachment,
    Inline,
}

pub struct Attachment {
    filename: Option<String>,
    content_disposition: Disposition,
    content_type: Option<Mime>,
    content_id: Option<String>,
}

impl Default for Attachment {
    fn default() -> Self {
        Self::new()
    }
}

impl Attachment {
    pub fn new() -> Self {
        Self {
            filename: None,
            content_disposition: Disposition::Attachment,
            content_type: None,
            content_id: None,
        }
    }

    pub fn new_inline() -> Self {
        Self {
            filename: None,
            content_disposition: Disposition::Inline,
            content_type: None,
            content_id: None,
        }
    }

    pub fn content_type(mut self, content_type: Mime) -> Self {
        self.content_type = Some(content_type);
        self
    }

    pub fn filename(mut self, filename: String) -> Self {
        self.filename = Some(filename);
        self
    }

    /// For use in inline attachments
    pub fn content_id(mut self, content_id: String) -> Self {
        self.content_id = Some(content_id);
        self
    }

    /// Build the attachment part
    pub fn body<T: IntoBody>(self, content: T) -> SinglePart {
        let mut builder = SinglePart::builder();

        builder = match self.content_disposition {
            Disposition::Attachment => match self.filename {
                Some(filename) => builder.header(header::ContentDisposition::attachment(&filename)),
                None => panic!("Missing filename attachment"),
            },
            Disposition::Inline => match self.filename {
                Some(filename) => {
                    builder.header(header::ContentDisposition::inline_with_name(&filename))
                }
                None => builder.header(header::ContentDisposition::inline()),
            },
        };

        if let Some(content_type) = self.content_type {
            builder = builder.header(header::ContentType::from_mime(content_type))
        }

        if let Some(content_id) = self.content_id {
            builder = builder.header(header::ContentId::parse(&content_id).unwrap())
        }

        builder.body(content)
    }
}
