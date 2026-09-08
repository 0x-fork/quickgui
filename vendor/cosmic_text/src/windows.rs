// SPDX-License-Identifier: MIT OR Apache-2.0

//! DirectWrite grayscale rasterization for the exact font bytes selected by the shaper.
//! Unsupported/color glyphs fall back to Swash. Native work happens only on atlas misses.

use crate::{CacheKey, CacheKeyFlags, FontSystem, HashMap, SwashContent, SwashImage};
use std::mem::ManuallyDrop;
use swash::{scale::Source, zeno::Placement};
use windows::{core::Interface, Win32::Graphics::DirectWrite::*};

const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
const MAX_FACES: usize = 128;

type FaceKey = (fontdb::ID, fontdb::Weight, u32, bool);

struct State {
    factory: IDWriteFactory6,
    loader: IDWriteInMemoryFontFileLoader,
    files: HashMap<fontdb::ID, IDWriteFontFile>,
    faces: HashMap<FaceKey, IDWriteFontFace5>,
    font_bytes: usize,
}

impl State {
    fn new() -> windows::core::Result<Self> {
        let factory: IDWriteFactory6 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_ISOLATED) }?;
        let loader = unsafe { factory.CreateInMemoryFontFileLoader() }?;
        unsafe { factory.RegisterFontFileLoader(&loader) }?;
        Ok(Self {
            factory,
            loader,
            files: HashMap::default(),
            faces: HashMap::default(),
            font_bytes: 0,
        })
    }

    fn face(&mut self, system: &mut FontSystem, key: CacheKey) -> Option<IDWriteFontFace5> {
        let fake_italic = key.flags.contains(CacheKeyFlags::FAKE_ITALIC);
        let face_key = (
            key.font_id,
            key.font_weight,
            key.optical_size_bits,
            fake_italic,
        );
        if let Some(face) = self.faces.get(&face_key) {
            return Some(face.clone());
        }
        let font = system.get_font_with_optical_size(
            key.font_id,
            key.font_weight,
            key.optical_size_bits,
        )?;
        if font.has_color_glyphs || font.data().len() > MAX_FONT_BYTES {
            return None;
        }
        let additional_bytes = if self.files.contains_key(&key.font_id) {
            0
        } else {
            font.data().len()
        };
        if self.faces.len() >= MAX_FACES || self.font_bytes + additional_bytes > MAX_FONT_BYTES {
            self.faces.clear();
            self.files.clear();
            self.font_bytes = 0;
        }
        let file = if let Some(file) = self.files.get(&key.font_id) {
            file.clone()
        } else {
            // A null owner asks DirectWrite to copy the bytes. Copy once per source face, reuse
            // for all weights/sizes, and keep total owned font bytes explicitly bounded.
            let file = unsafe {
                self.loader.CreateInMemoryFontFileReference(
                    &self.factory,
                    font.data().as_ptr().cast(),
                    font.data().len().try_into().ok()?,
                    None,
                )
            }
            .ok()?;
            self.font_bytes += font.data().len();
            self.files.insert(key.font_id, file.clone());
            file
        };
        let axes: Vec<_> = font
            .as_swash()
            .variations()
            .map(|axis| {
                let requested = match axis.tag() {
                    0x77676874 => f32::from(key.font_weight.0),
                    0x6f70737a if key.optical_size_bits != 0 => {
                        f32::from_bits(key.optical_size_bits)
                    }
                    _ => axis.default_value(),
                };
                DWRITE_FONT_AXIS_VALUE {
                    axisTag: DWRITE_FONT_AXIS_TAG(axis.tag().swap_bytes()),
                    value: requested.clamp(axis.min_value(), axis.max_value()),
                }
            })
            .collect();
        let simulation = if fake_italic {
            DWRITE_FONT_SIMULATIONS_OBLIQUE
        } else {
            DWRITE_FONT_SIMULATIONS_NONE
        };
        let reference = unsafe {
            self.factory
                .CreateFontFaceReference(&file, font.face_index(), simulation, &axes)
        }
        .ok()?;
        let face = unsafe { reference.CreateFontFace() }.ok()?;
        self.faces.insert(face_key, face.clone());
        Some(face)
    }

    fn image(&mut self, system: &mut FontSystem, key: CacheKey) -> Option<SwashImage> {
        let face = self.face(system, key)?;
        let size = f32::from_bits(key.font_size_bits);
        if !size.is_finite() || size <= 0.0 {
            return None;
        }
        let font_face = ManuallyDrop::new(Some(face.cast().ok()?));
        let run = DWRITE_GLYPH_RUN {
            fontFace: font_face,
            fontEmSize: size,
            glyphCount: 1,
            glyphIndices: &key.glyph_id,
            ..Default::default()
        };
        let mut mode = DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC;
        let mut grid = DWRITE_GRID_FIT_MODE_DEFAULT;
        let recommended = unsafe {
            face.GetRecommendedRenderingMode(
                size,
                96.0,
                96.0,
                None,
                false,
                DWRITE_OUTLINE_THRESHOLD_ANTIALIASED,
                DWRITE_MEASURING_MODE_NATURAL,
                None,
                &mut mode,
                &mut grid,
            )
        };
        if recommended.is_err() || mode == DWRITE_RENDERING_MODE1_OUTLINE {
            mode = DWRITE_RENDERING_MODE1_NATURAL_SYMMETRIC;
        }
        if key.flags.contains(CacheKeyFlags::DISABLE_HINTING) {
            grid = DWRITE_GRID_FIT_MODE_DISABLED;
        }
        let analysis = unsafe {
            self.factory.CreateGlyphRunAnalysis(
                &run,
                None,
                mode,
                DWRITE_MEASURING_MODE_NATURAL,
                grid,
                DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE,
                key.x_bin.as_float(),
                key.y_bin.as_float(),
            )
        };
        // DWRITE_GLYPH_RUN is a borrowed ABI structure, but this field owns our cloned COM ref.
        let mut run = run;
        unsafe {
            ManuallyDrop::drop(&mut run.fontFace);
        }
        let analysis = analysis.ok()?;
        let bounds = unsafe { analysis.GetAlphaTextureBounds(DWRITE_TEXTURE_ALIASED_1x1) }.ok()?;
        let width = u32::try_from(bounds.right.checked_sub(bounds.left)?).ok()?;
        let height = u32::try_from(bounds.bottom.checked_sub(bounds.top)?).ok()?;
        if width == 0 || height == 0 || width > 2048 || height > 2048 {
            return None;
        }
        let mut data = vec![0; width as usize * height as usize];
        unsafe { analysis.CreateAlphaTexture(DWRITE_TEXTURE_ALIASED_1x1, &bounds, &mut data) }
            .ok()?;
        Some(SwashImage {
            source: Source::Outline,
            content: SwashContent::Mask,
            placement: Placement {
                left: bounds.left,
                top: -bounds.top,
                width,
                height,
            },
            data,
        })
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.faces.clear();
        self.files.clear();
        let _ = unsafe { self.factory.UnregisterFontFileLoader(&self.loader) };
    }
}

#[derive(Default)]
pub(crate) struct Rasterizer {
    state: Option<Option<State>>,
}

impl Rasterizer {
    pub(crate) fn image(&mut self, system: &mut FontSystem, key: CacheKey) -> Option<SwashImage> {
        if !key.flags.contains(CacheKeyFlags::NATIVE_RASTERIZATION)
            || key
                .flags
                .intersects(CacheKeyFlags::PIXEL_FONT | CacheKeyFlags::FONT_THICKEN)
        {
            return None;
        }
        self.state
            .get_or_insert_with(|| State::new().ok())
            .as_mut()?
            .image(system, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directwrite_rasterizes_the_selected_in_memory_font_and_reuses_it() {
        let mut db = fontdb::Database::new();
        db.load_font_data(
            include_bytes!("../../../tests/fixtures/fonts/Inter-Regular.ttf").to_vec(),
        );
        let id = db.faces().next().unwrap().id;
        let mut system = FontSystem::new_with_locale_and_db("en-US".into(), db);
        let font = system.get_font(id, fontdb::Weight::NORMAL).unwrap();
        let glyph = font.as_swash().charmap().map('M');
        let (mut key, _, _) = CacheKey::new(
            id,
            glyph,
            14.0,
            (0.25, 0.0),
            fontdb::Weight::NORMAL,
            CacheKeyFlags::NATIVE_RASTERIZATION,
        );
        key.optical_size_bits = 14.0_f32.to_bits();
        let mut state = State::new().unwrap();
        let first = state.image(&mut system, key).unwrap();
        assert!(first.data.iter().any(|alpha| (1..255).contains(alpha)));
        let bytes = state.font_bytes;
        for weight in [
            fontdb::Weight::NORMAL,
            fontdb::Weight::MEDIUM,
            fontdb::Weight::BOLD,
        ] {
            let mut variant = key;
            variant.font_weight = weight;
            state.image(&mut system, variant).unwrap();
        }
        assert_eq!(state.files.len(), 1);
        assert_eq!(state.font_bytes, bytes);
        assert_eq!(state.image(&mut system, key).unwrap().data, first.data);
    }
}
