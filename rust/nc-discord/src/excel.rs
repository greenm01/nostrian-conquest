//! Excel (.xlsx) workbook generator for Nostrian Conquest starmaps.
//!
//! Converts raw CSV tactical starmap data and ASCII grid dumps into a formatted,
//! multi-sheet Excel spreadsheet compatible with Microsoft Excel, LibreOffice Calc,
//! Apple Numbers, and Google Sheets.

use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};

/// Build a formatted Excel (.xlsx) workbook buffer from starmap CSV data and optional ASCII grid text.
pub fn build_excel_starmap(
    csv_content: &str,
    ascii_grid: Option<&str>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut workbook = Workbook::new();

    // -----------------------------------------------------------------------
    // Sheet 1: Starmap Data (Formatted Table)
    // -----------------------------------------------------------------------
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Starmap Intel")?;
    worksheet.set_freeze_panes(1, 0)?;

    let header_format = Format::new()
        .set_bold()
        .set_font_name("Segoe UI")
        .set_font_size(11)
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x1F4E79)) // Deep Navy
        .set_align(FormatAlign::Center);

    let cell_format = Format::new()
        .set_font_name("Segoe UI")
        .set_font_size(10);

    let number_format = Format::new()
        .set_font_name("Segoe UI")
        .set_font_size(10)
        .set_align(FormatAlign::Right);

    for (row_idx, line) in csv_content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Split CSV row, handling simple quotes
        let fields: Vec<&str> = trimmed.split(',').map(|f| f.trim_matches('"')).collect();

        for (col_idx, field) in fields.iter().enumerate() {
            let r = row_idx as u32;
            let c = col_idx as u16;

            if row_idx == 0 {
                worksheet.write_string_with_format(r, c, *field, &header_format)?;
            } else if let Ok(val) = field.parse::<i64>() {
                worksheet.write_number_with_format(r, c, val as f64, &number_format)?;
            } else if let Ok(val) = field.parse::<f64>() {
                worksheet.write_number_with_format(r, c, val, &number_format)?;
            } else {
                worksheet.write_string_with_format(r, c, *field, &cell_format)?;
            }
        }
    }

    worksheet.autofit();

    // -----------------------------------------------------------------------
    // Sheet 2: Printable ASCII Grid (Optional)
    // -----------------------------------------------------------------------
    if let Some(grid) = ascii_grid {
        let sheet_grid = workbook.add_worksheet();
        sheet_grid.set_name("ASCII Starmap")?;

        let mono_format = Format::new()
            .set_font_name("Courier New")
            .set_font_size(9);

        for (r, line) in grid.lines().enumerate() {
            sheet_grid.write_string_with_format(r as u32, 0, line, &mono_format)?;
        }
        sheet_grid.autofit();
    }

    let buffer = workbook.save_to_buffer()?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_excel_starmap() {
        let csv = "Sector,X,Y,Name,Owner,Starbase\n1,10,20,Sol,1,Yes\n2,15,25,Capella,0,No\n";
        let ascii = "  1 2 3\n1 * . .\n2 . * .\n";

        let buf = build_excel_starmap(csv, Some(ascii)).expect("workbook built");
        assert!(!buf.is_empty());
        // Verify PK zip header of xlsx
        assert_eq!(&buf[0..2], b"PK");
    }
}
