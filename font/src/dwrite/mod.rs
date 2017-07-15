// Copyright 2016 Joe Wilm, The Alacritty Project Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//! Rasterization powered by dwrite bindings for windows using the dwrote bindings from https://github.com/vvuk/dwrote-rs.
// Created by Ray Imber (@rayman22201)
// Thanks to @rigtorp for the inspiration, and Joe Wilm for this awesome project.
// @see: https://github.com/jwilm/alacritty/issues/28

use std::collections::HashMap;
use super::{FontDesc, RasterizedGlyph, Metrics, Size, FontKey, GlyphKey, Weight, Slant, Style};
use dwrote::{FontCollection, FontFace, FontWeight, FontStretch, FontStyle, RenderingParams, GdiInterop, DWRITE_MEASURING_MODE_NATURAL, GlyphOffset};

/// Rasterizes glyphs for a single font face.
pub struct DwroteRasterizer {
    library: FontCollection,
    faces: HashMap<FontKey, FontFace>,
    keys: HashMap<FontDesc, FontKey>,
    dpi_x: u32,
    dpi_y: u32,
    dpr: f32,
}

impl ::Rasterize for DwroteRasterizer {
    type Err = Error;

    fn new(dpi_x: f32, dpi_y: f32, device_pixel_ratio: f32, _: bool) -> Result<DwroteRasterizer, Error> {
        Ok(DwroteRasterizer {
            library: FontCollection::system(),
            faces: HashMap::new(),
            keys: HashMap::new(),
            dpi_x: dpi_x as u32,
            dpi_y: dpi_y as u32,
            dpr: device_pixel_ratio,
        })
    }

    fn metrics(&self, key: FontKey) -> Result<Metrics, Error> {
        let face = self.faces
            .get(&key)
            .ok_or(Error::FontNotLoaded)?;

        let fm = face.metrics();
        // I can't find an "average" metric, so this is hack that just gets the metrics for 'A'
        let a_index = face.get_glyph_indices(&['A' as u32])[0];
        let gm = face.get_design_glyph_metrics(&[a_index], false)[0];


        let scale_size = self.dpr as f64;

        let em_size = fm.designUnitsPerEm as f64;
        let w = gm.advanceWidth as f64;
        let h = (fm.ascent - fm.descent + fm.capHeight) as f64;

        let w_scale = w * scale_size / em_size;
        let h_scale = h * scale_size / em_size;
        let d_scale = (fm.descent as f64) * scale_size / em_size;

        Ok(Metrics {
            average_advance: w_scale,
            line_height: h_scale,
            descent: d_scale as f32
        })
    }

    fn load_font(&mut self, desc: &FontDesc, _size: Size) -> Result<FontKey, Error> {
        self.keys
            .get(&desc.to_owned())
            .map(|k| Ok(*k))
            .unwrap_or_else(|| {
                let face = self.get_face(desc)?;
                let key = FontKey::next();
                self.faces.insert(key, face);
                Ok(key)
            })
    }

    fn get_glyph(&mut self, glyph_key: &GlyphKey) -> Result<RasterizedGlyph, Error> {
        let face = self.faces
            .get(&glyph_key.font_key)
            .ok_or(Error::FontNotLoaded)?;

        let size = glyph_key.size.as_f32_pts() * self.dpr;
        let c = glyph_key.c;
        let c_index = face.get_glyph_indices(&[c as u32])[0];
        let gm = face.get_design_glyph_metrics(&[c_index], false)[0];

        let design_units_per_pixel = face.metrics().designUnitsPerEm as f32 / 16. as f32;
        let scaled_design_units_to_pixels = size / design_units_per_pixel;

        let width = (gm.advanceWidth as i32 - (gm.leftSideBearing + gm.rightSideBearing)) as f32 * scaled_design_units_to_pixels;
        let height = (gm.advanceHeight as i32 - (gm.topSideBearing + gm.bottomSideBearing)) as f32 * scaled_design_units_to_pixels;
        let x = (-gm.leftSideBearing) as f32 * scaled_design_units_to_pixels;
        let y = (gm.verticalOriginY - gm.topSideBearing) as f32 * scaled_design_units_to_pixels;

        let gdi_interop = GdiInterop::create();
        let rt = gdi_interop.create_bitmap_render_target(width as u32, height as u32);
        let rp = RenderingParams::create_for_primary_monitor();
        rt.set_pixels_per_dip(self.dpr);
        //let em_size = 10.0f32; // pulled this value from dwrite, but I'm not sure if it's correct. It's kind of a magic number...
        rt.draw_glyph_run(x as f32, y as f32,
                          DWRITE_MEASURING_MODE_NATURAL,
                          &face,
                          size,
                          &[c_index],
                          &[0f32],
                          &[GlyphOffset { advanceOffset: 0., ascenderOffset: 0. }],
                          &rp,
                          &(255.0f32, 255.0f32, 255.0f32));
        let bytes = rt.get_opaque_values_as_mask();

        Ok(RasterizedGlyph {
            c: c,
            top: y as i32,
            left: x as i32,
            width: width as i32,
            height: width as i32,
            buf: bytes,
        })
    }
}

impl DwroteRasterizer {
    /// Load a font face accoring to `FontDesc`
    fn get_face(&mut self, desc: &FontDesc) -> Result<FontFace, Error> {
        match desc.style {
            Style::Description { slant, weight } => {
                // Match nearest font
                self.get_matching_face(&desc, slant, weight)
            }
            Style::Specific(ref style) => {
                // If a name was specified, try and load specifically that font.
                self.get_specific_face(&desc, &style)
            }
        }
    }

    fn get_matching_face(
        &mut self,
        desc: &FontDesc,
        slant: Slant,
        weight: Weight
    ) -> Result<FontFace, Error> {
        let family = self.library.get_font_family_by_name(&desc.name).unwrap();
        //map slant to FontStyle and weight to FontWeight
        let font_style = match slant {
            Slant::Normal   => FontStyle::Normal,
            Slant::Italic   => FontStyle::Italic,
            Slant::Oblique  => FontStyle::Oblique
        };
        let font_weight = match weight {
            Weight::Normal  => FontWeight::Regular,
            Weight::Bold    => FontWeight::Bold,
        };
        // I want to use panic::catch_unwind, but dwrote does not support it
        Ok(family.get_first_matching_font(font_weight, FontStretch::Normal, font_style).create_font_face())
    }

    fn get_specific_face(
        &mut self,
        desc: &FontDesc,
        style: &str
    ) -> Result<FontFace, Error> {
        let family = self.library.get_font_family_by_name(&desc.name).unwrap();
        // parse style into either Normal, Bold, or Italic
        // I guess this is how specific face is supposed to work? idk for sure...
        let font_info = match style {
            "Normal"    => (FontWeight::Regular, FontStretch::Normal, FontStyle::Normal),
            "Bold"      => (FontWeight::Bold, FontStretch::Normal, FontStyle::Normal),
            "Italic"    => (FontWeight::Regular, FontStretch::Normal, FontStyle::Italic),
            &_          => (FontWeight::Regular, FontStretch::Normal, FontStyle::Normal),
        };
        // I want to use panic::catch_unwind, but dwrote does not support it.
        Ok(family.get_first_matching_font(font_info.0, font_info.1, font_info.2).create_font_face())
    }
}

/// Errors occurring when using the directwrite rasterizer
#[derive(Debug)]
pub enum Error {
    /// Couldn't find font matching description
    MissingFont(FontDesc),

    /// Requested an operation with a FontKey that isn't known to the rasterizer
    FontNotLoaded,
}

impl ::std::error::Error for Error {
    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }

    fn description(&self) -> &str {
        match *self {
            Error::MissingFont(ref _desc) => "couldn't find the requested font",
            Error::FontNotLoaded => "tried to operate on font that hasn't been loaded",
        }
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Error::MissingFont(ref desc) => {
                write!(f, "Couldn't find a font with {}\
                       \n\tPlease check the font config in your alacritty.yml.", desc)
            },
            Error::FontNotLoaded => {
                f.write_str("Tried to use a font that hasn't been loaded")
            }
        }
    }
}
