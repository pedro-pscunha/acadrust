//! Private DWG-bridge transport for a surface's native modeler cache, separate from its SAB.
use crate::entities::Surface;
use crate::error::{DxfError, Result};
use crate::io::dwg::dwg_stream_readers::{
    bit_reader::DwgBitReader, merged_reader::DwgMergedReader,
    object_reader::entities::read_acis_entity,
};
use crate::io::dwg::DwgVersion;
use crate::types::{DxfVersion, Handle};

pub(crate) const SUBCLASS: &str = "AcDbAcadrustSurfaceCache";
pub(crate) const MAX_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn append_hex(data: &mut Vec<u8>, value: &str) -> Result<()> {
    let value = value.trim().as_bytes();
    if !value.len().is_multiple_of(2) || value.len() / 2 > MAX_BYTES.saturating_sub(data.len()) {
        return Err(DxfError::Parse(
            "Invalid private surface cache byte count".into(),
        ));
    }
    let bytes: Result<Vec<_>> = value
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16);
            let low = (pair[1] as char).to_digit(16);
            high.zip(low)
                .map(|(high, low)| (high * 16 + low) as u8)
                .ok_or_else(|| DxfError::Parse("Invalid private surface cache hex data".into()))
        })
        .collect();
    data.extend(bytes?);
    Ok(())
}

pub(crate) fn restore(data: &[u8], materials: &[Handle], surface: &mut Surface) -> Result<()> {
    let invalid = || DxfError::Parse("Invalid private surface modeler cache".into());
    let main_bits = data.get(..8).ok_or_else(invalid)?;
    let handle_bits = data.get(8..16).ok_or_else(invalid)?;
    let main_bits = usize::try_from(u64::from_le_bytes(
        main_bits.try_into().map_err(|_| invalid())?,
    ))
    .map_err(|_| invalid())?;
    let handle_bits = usize::try_from(u64::from_le_bytes(
        handle_bits.try_into().map_err(|_| invalid())?,
    ))
    .map_err(|_| invalid())?;
    let main_len = main_bits.div_ceil(8);
    let handle_len = handle_bits.div_ceil(8);
    let body = data.get(16..).ok_or_else(invalid)?;
    if main_len.checked_add(handle_len) != Some(body.len()) || data.len() > MAX_BYTES {
        return Err(invalid());
    }
    let version = DxfVersion::AC1032;
    let main = DwgBitReader::new(body[..main_len].to_vec(), DwgVersion::AC24, version);
    let handles = DwgBitReader::new(body[main_len..].to_vec(), DwgVersion::AC24, version);
    let mut reader = DwgMergedReader::from_readers(main, None, Some(handles), version);
    let end = i64::try_from(main_bits).map_err(|_| invalid())?;
    reader.set_main_data_end(end);
    reader.set_handle_start(end);
    let cache = read_acis_entity(&mut reader, DwgVersion::AC24, version, true);
    reader.check_error()?;
    if reader.position_in_bits() != end
        || reader.handle_remaining_bits() != ((8 - handle_bits % 8) % 8) as i64
        || cache.materials.len() != materials.len()
    {
        return Err(invalid());
    }
    surface.point_of_reference = cache.point;
    surface.wires = cache.wires;
    surface.silhouettes = cache.silhouettes;
    surface.acis_data.revision = cache.revision;
    surface.acis_data.materials = cache.materials;
    for (material, handle) in surface.acis_data.materials.iter_mut().zip(materials) {
        material.material_handle = (!handle.is_null()).then_some(*handle);
    }
    surface.acis_data.wireframe_data_present = cache.wireframe_data_present;
    surface.acis_data.wireframe_point_present = cache.wireframe_point_present;
    surface.acis_data.wireframe_isoline_present = cache.wireframe_isoline_present;
    surface.acis_data.acis_empty_bit = cache.acis_empty_bit;
    surface.acis_data.extra_acis_data = cache.extra_acis_data.map(Box::new);
    surface.acis_data.wireframe_isolines = cache.isolines;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::SurfaceKind;

    #[test]
    fn malformed_cache_hex_does_not_change_the_buffer() {
        for value in ["0", "GG", "00GG", "é"] {
            let mut data = vec![1, 2];
            assert!(append_hex(&mut data, value).is_err());
            assert_eq!(data, vec![1, 2]);
        }
    }

    #[test]
    fn malformed_cache_framing_is_rejected_before_native_decode() {
        for data in [Vec::new(), vec![0; 15], vec![255; 16]] {
            assert!(restore(&data, &[], &mut Surface::new(SurfaceKind::Generic)).is_err());
        }
    }
}
