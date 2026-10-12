//! Map downloads built from one authorized player projection.
use crate::admin::Result;
use nc_data::PlayerStarmapProjection;
use rust_xlsxwriter::{Format, Workbook};
use std::io::{Cursor, Write};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn rows(csv: &str) -> Result<Vec<Vec<String>>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv.as_bytes())
        .records()
        .map(|record| Ok(record?.iter().map(str::to_owned).collect()))
        .collect()
}

fn excel(sheets: &[(&str, Vec<Vec<String>>)]) -> Result<Vec<u8>> {
    let mut book = Workbook::new();
    let header = Format::new().set_bold();
    let cell = Format::new().set_font_name("Courier New").set_font_size(10);
    for (name, rows) in sheets {
        let sheet = book.add_worksheet();
        sheet.set_name(*name)?;
        sheet.set_freeze_panes(1, 1)?;
        for (r, row) in rows.iter().enumerate() {
            for (c, text) in row.iter().enumerate() {
                // Names are always strings, never spreadsheet formulas.
                sheet.write_string_with_format(
                    r as u32,
                    c as u16,
                    text,
                    if r == 0 { &header } else { &cell },
                )?;
            }
        }
        sheet.autofit();
    }
    Ok(book.save_to_buffer()?)
}

fn xml(text: &str) -> String {
    text.chars()
        .filter(|c| {
            matches!(*c, '\t' | '\n' | '\r')
                || (*c >= ' ' && !matches!(*c, '\u{fffe}' | '\u{ffff}'))
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn add(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    contents: &[u8],
    method: CompressionMethod,
) -> Result<()> {
    zip.start_file(
        name,
        SimpleFileOptions::default().compression_method(method),
    )?;
    zip.write_all(contents)?;
    Ok(())
}

fn ods(sheets: &[(&str, Vec<Vec<String>>)]) -> Result<Vec<u8>> {
    // ODF 1.2 package: mimetype must be first and uncompressed.
    // https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-part3.html
    let mime = "application/vnd.oasis.opendocument.spreadsheet";
    let mut content = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.2"><office:body><office:spreadsheet>"#,
    );
    for (name, rows) in sheets {
        content.push_str(&format!("<table:table table:name=\"{}\">", xml(name)));
        for row in rows {
            content.push_str("<table:table-row>");
            for text in row {
                content.push_str(&format!("<table:table-cell office:value-type=\"string\"><text:p>{}</text:p></table:table-cell>", xml(text)));
            }
            content.push_str("</table:table-row>");
        }
        content.push_str("</table:table>");
    }
    content.push_str("</office:spreadsheet></office:body></office:document-content>");
    let manifest = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2"><manifest:file-entry manifest:full-path="/" manifest:media-type="{mime}" manifest:version="1.2"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/></manifest:manifest>"#
    );
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    add(
        &mut zip,
        "mimetype",
        mime.as_bytes(),
        CompressionMethod::Stored,
    )?;
    add(
        &mut zip,
        "content.xml",
        content.as_bytes(),
        CompressionMethod::Deflated,
    )?;
    add(
        &mut zip,
        "META-INF/manifest.xml",
        manifest.as_bytes(),
        CompressionMethod::Deflated,
    )?;
    Ok(zip.finish()?.into_inner())
}

/// All formats share this projection's empire, year and fog-of-war information.
pub fn render(projection: &PlayerStarmapProjection, format: &str) -> Result<Vec<u8>> {
    let grid = projection.render_csv_export();
    let details = projection.render_csv_details_export();
    let sheets = [("Starmap", rows(&grid)?), ("Known Worlds", rows(&details)?)];
    match format {
        "map-xlsx" => excel(&sheets),
        "map-ods" => ods(&sheets),
        "map-zip" => {
            let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
            for (name, contents) in [
                ("starmap.txt", projection.render_ascii_export().into_bytes()),
                ("starmap.csv", grid.into_bytes()),
                ("starmap-DETAILS.csv", details.into_bytes()),
                ("starmap.ods", ods(&sheets)?),
                ("starmap.xlsx", excel(&sheets)?),
            ] {
                add(&mut zip, name, &contents, CompressionMethod::Deflated)?;
            }
            Ok(zip.finish()?.into_inner())
        }
        _ => Err("unsupported map archive format".into()),
    }
}

/// The JSON admin protocol encodes binary downloads; text exports stay unchanged.
pub fn download_bytes(data: &serde_json::Value) -> Result<Vec<u8>> {
    use base64::Engine;
    if let Some(encoded) = data["contents_base64"].as_str() {
        Ok(base64::engine::general_purpose::STANDARD.decode(encoded)?)
    } else {
        Ok(data["contents"]
            .as_str()
            .ok_or("missing download contents")?
            .as_bytes()
            .to_vec())
    }
}
