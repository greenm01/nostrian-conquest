use std::path::PathBuf;

use nc_data::ProductionItemKind;

use super::paths::resolve_repo_path;

pub(crate) fn parse_u8_arg(value: &str, label: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16)?
    } else {
        value.parse::<u8>()?
    };
    let _ = label;
    Ok(parsed)
}

pub(crate) fn parse_u16_arg(value: &str, label: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16)?
    } else {
        value.parse::<u16>()?
    };
    let _ = label;
    Ok(parsed)
}

pub(crate) fn parse_usize_1_based(
    value: &str,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let parsed = value.parse::<usize>()?;
    if parsed == 0 {
        return Err(format!("{label} must be >= 1").into());
    }
    Ok(parsed)
}

pub(crate) fn parse_optional_source_and_target(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf)> {
    match args.as_slice() {
        [target] => Some((default_source, PathBuf::from(target))),
        [source, target] => Some((resolve_repo_path(source), PathBuf::from(target))),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_name(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, String)> {
    match args.as_slice() {
        [target, scenario_name] => {
            Some((default_source, PathBuf::from(target), scenario_name.clone()))
        }
        [source, target, scenario_name] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            scenario_name.clone(),
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_xy(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u8, u8)> {
    match args.as_slice() {
        [target, x, y] => Some((
            default_source,
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
        )),
        [source, target, x, y] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_count(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u16)> {
    match args.as_slice() {
        [target, count] => Some((
            default_source,
            PathBuf::from(target),
            count.parse::<u16>().ok()?,
        )),
        [source, target, count] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            count.parse::<u16>().ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_count_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<u16>)> {
    match args.as_slice() {
        [target_root, counts @ ..] if !counts.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            counts
                .iter()
                .map(|value| value.parse::<u16>().ok())
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, counts @ ..] if !counts.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            counts
                .iter()
                .map(|value| value.parse::<u16>().ok())
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_fleet_spec(
    args: Vec<String>,
) -> Option<(PathBuf, usize, u8, u8, u8, u8, Option<u8>, Option<u8>)> {
    match args.as_slice() {
        [target, record_index, speed, order_code, target_x, target_y] => Some((
            PathBuf::from(target),
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            None,
            None,
        )),
        [
            target,
            record_index,
            speed,
            order_code,
            target_x,
            target_y,
            aux0,
        ] => Some((
            PathBuf::from(target),
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            Some(parse_u8_arg(aux0, "aux0").ok()?),
            None,
        )),
        [
            target,
            record_index,
            speed,
            order_code,
            target_x,
            target_y,
            aux0,
            aux1,
        ] => Some((
            PathBuf::from(target),
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            Some(parse_u8_arg(aux0, "aux0").ok()?),
            Some(parse_u8_arg(aux1, "aux1").ok()?),
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_planet_spec(
    args: Vec<String>,
) -> Option<(PathBuf, usize, u32, ProductionItemKind)> {
    match args.as_slice() {
        [target, record_index, quantity, kind] => Some((
            PathBuf::from(target),
            parse_usize_1_based(record_index, "planet record index").ok()?,
            quantity.parse().ok()?,
            ProductionItemKind::from_name(kind)?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_coord_pair(value: &str) -> Option<[u8; 2]> {
    let (x, y) = value.split_once(':')?;
    Some([
        parse_u8_arg(x, "target_x").ok()?,
        parse_u8_arg(y, "target_y").ok()?,
    ])
}

pub(crate) fn parse_optional_source_target_and_coord_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<[u8; 2]>)> {
    match args.as_slice() {
        [target_root, coords @ ..] if !coords.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            coords
                .iter()
                .map(|value| parse_coord_pair(value))
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, coords @ ..] if !coords.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            coords
                .iter()
                .map(|value| parse_coord_pair(value))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_fleet_order_spec(
    value: &str,
) -> Option<(usize, u8, u8, u8, u8, Option<u8>, Option<u8>)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [record_index, speed, order_code, target_x, target_y] => Some((
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            None,
            None,
        )),
        [record_index, speed, order_code, target_x, target_y, aux0] => Some((
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            Some(parse_u8_arg(aux0, "aux0").ok()?),
            None,
        )),
        [
            record_index,
            speed,
            order_code,
            target_x,
            target_y,
            aux0,
            aux1,
        ] => Some((
            parse_usize_1_based(record_index, "fleet record index").ok()?,
            parse_u8_arg(speed, "speed").ok()?,
            parse_u8_arg(order_code, "order code").ok()?,
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            Some(parse_u8_arg(aux0, "aux0").ok()?),
            Some(parse_u8_arg(aux1, "aux1").ok()?),
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_fleet_spec_list(
    args: Vec<String>,
) -> Option<(
    PathBuf,
    Vec<(usize, u8, u8, u8, u8, Option<u8>, Option<u8>)>,
)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|value| parse_fleet_order_spec(value))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_bombard_spec(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u8, u8, u16, u16)> {
    match args.as_slice() {
        [target, target_x, target_y, ca, dd] => Some((
            default_source,
            PathBuf::from(target),
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
        )),
        [source, target, target_x, target_y, ca, dd] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_bombard_spec(value: &str) -> Option<(u8, u8, u16, u16)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [target_x, target_y, ca, dd] => Some((
            parse_u8_arg(target_x, "target_x").ok()?,
            parse_u8_arg(target_y, "target_y").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_bombard_spec_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<(u8, u8, u16, u16)>)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_bombard_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, specs @ ..] if !specs.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_bombard_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_invade_spec(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u8, u8, u8, u16, u16, u16, u16, u8)> {
    match args.as_slice() {
        [target, x, y, sc, bb, ca, dd, tt, armies] => Some((
            default_source,
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u8_arg(sc, "sc").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u16_arg(tt, "tt").ok()?,
            parse_u8_arg(armies, "armies").ok()?,
        )),
        [source, target, x, y, sc, bb, ca, dd, tt, armies] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u8_arg(sc, "sc").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u16_arg(tt, "tt").ok()?,
            parse_u8_arg(armies, "armies").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_invade_spec(value: &str) -> Option<(u8, u8, u8, u16, u16, u16, u16, u8)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [x, y, sc, bb, ca, dd, tt, armies] => Some((
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u8_arg(sc, "sc").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u16_arg(tt, "tt").ok()?,
            parse_u8_arg(armies, "armies").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_invade_spec_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<(u8, u8, u8, u16, u16, u16, u16, u8)>)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_invade_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, specs @ ..] if !specs.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_invade_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_fleet_battle_spec(
    value: &str,
) -> Option<(
    u8,
    u8,
    u8,
    u16,
    u16,
    u16,
    u16,
    u16,
    u8,
    u16,
    u16,
    u8,
    u8,
    u8,
    u16,
    u16,
    u8,
    u8,
    u8,
    u8,
)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [
            bx,
            by,
            f0r,
            f0bb,
            f0ca,
            f0dd,
            f2ca,
            f2dd,
            f4sc,
            f4bb,
            f4ca,
            f8lx,
            f8ly,
            f8sc,
            f8bb,
            f8ca,
            p14x,
            p14y,
            p14a,
            p14b,
        ] => Some((
            parse_u8_arg(bx, "battle_x").ok()?,
            parse_u8_arg(by, "battle_y").ok()?,
            parse_u8_arg(f0r, "f0_roe").ok()?,
            parse_u16_arg(f0bb, "f0_bb").ok()?,
            parse_u16_arg(f0ca, "f0_ca").ok()?,
            parse_u16_arg(f0dd, "f0_dd").ok()?,
            parse_u16_arg(f2ca, "f2_ca").ok()?,
            parse_u16_arg(f2dd, "f2_dd").ok()?,
            parse_u8_arg(f4sc, "f4_sc").ok()?,
            parse_u16_arg(f4bb, "f4_bb").ok()?,
            parse_u16_arg(f4ca, "f4_ca").ok()?,
            parse_u8_arg(f8lx, "f8_loc_x").ok()?,
            parse_u8_arg(f8ly, "f8_loc_y").ok()?,
            parse_u8_arg(f8sc, "f8_sc").ok()?,
            parse_u16_arg(f8bb, "f8_bb").ok()?,
            parse_u16_arg(f8ca, "f8_ca").ok()?,
            parse_u8_arg(p14x, "p14_x").ok()?,
            parse_u8_arg(p14y, "p14_y").ok()?,
            parse_u8_arg(p14a, "p14_armies").ok()?,
            parse_u8_arg(p14b, "p14_batteries").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_fleet_battle_spec_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(
    PathBuf,
    PathBuf,
    Vec<(
        u8,
        u8,
        u8,
        u16,
        u16,
        u16,
        u16,
        u16,
        u8,
        u16,
        u16,
        u8,
        u8,
        u8,
        u16,
        u16,
        u8,
        u8,
        u8,
        u8,
    )>,
)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_fleet_battle_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, specs @ ..] if !specs.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_fleet_battle_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_econ_spec(value: &str) -> Option<(u8, u8, u16, u16, u16, u8, u8, u8, u8)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [x, y, bb, ca, dd, p14x, p14y, p14a, p14b] => Some((
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u8_arg(p14x, "p14_x").ok()?,
            parse_u8_arg(p14y, "p14_y").ok()?,
            parse_u8_arg(p14a, "p14_armies").ok()?,
            parse_u8_arg(p14b, "p14_batteries").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_econ_spec(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u8, u8, u16, u16, u16, u8, u8, u8, u8)> {
    match args.as_slice() {
        [target, x, y, bb, ca, dd, p14x, p14y, p14a, p14b] => Some((
            default_source,
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u8_arg(p14x, "p14_x").ok()?,
            parse_u8_arg(p14y, "p14_y").ok()?,
            parse_u8_arg(p14a, "p14_armies").ok()?,
            parse_u8_arg(p14b, "p14_batteries").ok()?,
        )),
        [source, target, x, y, bb, ca, dd, p14x, p14y, p14a, p14b] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
            parse_u16_arg(bb, "bb").ok()?,
            parse_u16_arg(ca, "ca").ok()?,
            parse_u16_arg(dd, "dd").ok()?,
            parse_u8_arg(p14x, "p14_x").ok()?,
            parse_u8_arg(p14y, "p14_y").ok()?,
            parse_u8_arg(p14a, "p14_armies").ok()?,
            parse_u8_arg(p14b, "p14_batteries").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_econ_spec_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(
    PathBuf,
    PathBuf,
    Vec<(u8, u8, u16, u16, u16, u8, u8, u8, u8)>,
)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_econ_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, specs @ ..] if !specs.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|v| parse_econ_spec(v))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_planet_build_spec(value: &str) -> Option<(usize, u32, ProductionItemKind)> {
    let parts = value.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [record_index, quantity, kind] => Some((
            parse_usize_1_based(record_index, "planet record index").ok()?,
            quantity.parse().ok()?,
            ProductionItemKind::from_name(kind)?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_target_and_planet_spec_list(
    args: Vec<String>,
) -> Option<(PathBuf, Vec<(usize, u32, ProductionItemKind)>)> {
    match args.as_slice() {
        [target_root, specs @ ..] if !specs.is_empty() => Some((
            PathBuf::from(target_root),
            specs
                .iter()
                .map(|value| parse_planet_build_spec(value))
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}
