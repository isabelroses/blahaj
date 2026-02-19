use color_eyre::eyre::{Result, eyre};

use poise::CreateReply;
use poise::serenity_prelude::CreateAttachment;

use std::sync::OnceLock;

use typst::Library;
use typst::LibraryExt;
use typst::World;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst_kit::fonts::FontSearcher;

use crate::types::Context;

struct FontState {
    book: LazyHash<FontBook>,
    fonts: Vec<typst_kit::fonts::FontSlot>,
}

static FONTS: OnceLock<FontState> = OnceLock::new();

fn fonts() -> &'static FontState {
    FONTS.get_or_init(|| {
        let fonts = FontSearcher::new().include_system_fonts(false).search();
        FontState {
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
        }
    })
}

struct MathWorld {
    source: Source,
    library: LazyHash<Library>,
}

impl MathWorld {
    fn new(expression: &str) -> Self {
        let text = format!(
            "#set page(width: auto, height: auto, margin: 10pt, fill: white)\n\
             #set text(size: 20pt)\n\
             $ {expression} $\n"
        );

        Self {
            source: Source::detached(text),
            library: LazyHash::new(Library::default()),
        }
    }
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &fonts().book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn typst(
    ctx: Context<'_>,
    #[description = "typst math expression"] expression: String,
) -> Result<()> {
    ctx.defer().await?;

    let result = tokio::task::spawn_blocking(move || {
        let world = MathWorld::new(&expression);
        let document = typst::compile::<PagedDocument>(&world)
            .output
            .map_err(|diagnostics| {
                let messages: Vec<String> =
                    diagnostics.iter().map(|d| d.message.to_string()).collect();
                eyre!("Compilation error:\n{}", messages.join("\n"))
            })?;

        let page = &document.pages[0];
        let pixmap = typst_render::render(page, 9.0);
        pixmap
            .encode_png()
            .map_err(|e| eyre!("PNG encoding failed: {e}"))
    })
    .await??;

    let attachment = CreateAttachment::bytes(result, "math.png");
    ctx.send(CreateReply::default().attachment(attachment))
        .await?;

    Ok(())
}
