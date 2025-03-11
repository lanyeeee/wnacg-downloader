use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Context;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::{config::Config, events::ExportPdfEvent, types::Comic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Archive {
    Pdf,
}

impl Archive {
    pub fn extension(self) -> &'static str {
        match self {
            Archive::Pdf => "pdf",
        }
    }
}

pub fn pdf(app: &AppHandle, comic: &Comic) -> anyhow::Result<()> {
    let title = &comic.title;
    let event_uuid = uuid::Uuid::new_v4().to_string();
    // 发送开始创建pdf事件
    let _ = ExportPdfEvent::Start {
        uuid: event_uuid.clone(),
        title: title.clone(),
    }
    .emit(app);
    let comic_download_dir = get_comic_download_dir(app, comic);
    let comic_export_dir = get_comic_export_dir(app, comic);
    // 保证导出目录存在
    std::fs::create_dir_all(&comic_export_dir)
        .context(format!("创建目录`{comic_export_dir:?}`失败"))?;
    // 创建pdf
    let extension = Archive::Pdf.extension();
    let pdf_path = comic_export_dir.join(format!("{title}.{extension}"));
    create_pdf(&comic_download_dir, &pdf_path).context("创建pdf失败")?;
    // 发送创建pdf完成事件
    let _ = ExportPdfEvent::End { uuid: event_uuid }.emit(app);
    Ok(())
}

/// 用`comic_download_dir`中的图片创建PDF，保存到`pdf_path`中
#[allow(clippy::similar_names)]
#[allow(clippy::cast_possible_truncation)]
fn create_pdf(comic_download_dir: &Path, pdf_path: &Path) -> anyhow::Result<()> {
    let mut image_paths = std::fs::read_dir(comic_download_dir)
        .context(format!("读取目录`{comic_download_dir:?}`失败"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() != Some(OsStr::new("json"))) // 过滤掉元数据.json文件
        .collect::<Vec<_>>();
    image_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids = vec![];

    for image_path in image_paths {
        if !image_path.is_file() {
            continue;
        }

        let buffer = read_image_to_buffer(&image_path)
            .context(format!("将`{image_path:?}`读取到buffer失败"))?;
        let (width, height) = image::image_dimensions(&image_path)
            .context(format!("获取`{image_path:?}`的尺寸失败"))?;
        let image_stream = lopdf::xobject::image_from(buffer)
            .context(format!("创建`{image_path:?}`的图片流失败"))?;
        // 将图片流添加到doc中
        let img_id = doc.add_object(image_stream);
        // 图片的名称，用于 Do 操作在页面上显示图片
        let img_name = format!("X{}", img_id.0);
        // 用于设置图片在页面上的位置和大小
        let cm_operation = Operation::new(
            "cm",
            vec![
                width.into(),
                0.into(),
                0.into(),
                height.into(),
                0.into(),
                0.into(),
            ],
        );
        // 用于显示图片
        let do_operation = Operation::new("Do", vec![Object::Name(img_name.as_bytes().to_vec())]);
        // 创建页面，设置图片的位置和大小，然后显示图片
        // 因为是从零开始创建PDF，所以没必要用 q 和 Q 操作保存和恢复图形状态
        let content = Content {
            operations: vec![cm_operation, do_operation],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });
        // 将图片以 XObject 的形式添加到文档中
        // Do 操作只能引用 XObject(所以前面定义的 Do 操作的参数是 img_name, 而不是 img_id)
        doc.add_xobject(page_id, img_name.as_bytes(), img_id)?;
        // 记录新创建的页面的 ID
        page_ids.push(page_id);
    }
    // 将"Pages"添加到doc中
    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as u32,
        "Kids" => page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    // 新建一个"Catalog"对象，将"Pages"对象添加到"Catalog"对象中，然后将"Catalog"对象添加到doc中
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc.compress();

    doc.save(pdf_path)
        .context(format!("保存`{pdf_path:?}`失败"))?;
    Ok(())
}

/// 读取`image_path`中的图片数据到buffer中
fn read_image_to_buffer(image_path: &Path) -> anyhow::Result<Vec<u8>> {
    let file = std::fs::File::open(image_path).context(format!("打开`{image_path:?}`失败"))?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![];
    reader
        .read_to_end(&mut buffer)
        .context(format!("读取`{image_path:?}`失败"))?;
    Ok(buffer)
}

fn get_comic_download_dir(app: &AppHandle, comic: &Comic) -> PathBuf {
    app.state::<RwLock<Config>>()
        .read()
        .download_dir
        .join(&comic.title)
}

fn get_comic_export_dir(app: &AppHandle, comic: &Comic) -> PathBuf {
    app.state::<RwLock<Config>>()
        .read()
        .export_dir
        .join(&comic.title)
}
