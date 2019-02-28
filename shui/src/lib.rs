#![allow(dead_code)]

// renderer specific modules
#[cfg(feature = "ogl")]
#[path="cx_ogl.rs"]
mod cx; 
#[cfg(feature = "ogl")]
#[path="cxdrawing_ogl.rs"]
mod cxdrawing; 
#[cfg(feature = "ogl")]
#[path="cxtextures_ogl.rs"]
mod cxtextures;

#[cfg(feature = "mtl")]
#[path="cx_mtl.rs"]
mod cx; 
#[cfg(feature = "mtl")]
#[path="cxdrawing_mtl.rs"]
mod cxdrawing; 
#[cfg(feature = "mtl")]
#[path="cxtextures_mtl.rs"]
mod cxtextures;

#[cfg(feature = "webgl")]
#[path="cx_webgl.rs"]
mod cx; 
#[cfg(feature = "webgl")]
#[path="cxdrawing_webgl.rs"]
mod cxdrawing; 
#[cfg(feature = "webgl")]
#[path="cxtextures_webgl.rs"]
mod cxtextures;

#[cfg(any(feature = "webgl", feature = "ogl"))]
mod cxdrawing_gl; 

#[cfg(any(feature = "mtl", feature = "ogl"))]
mod cx_winit; 

mod cx_shared;

// shared modules
mod cxdrawing_shared;
mod cxfonts;
mod cxturtle;
mod animation;
mod elements;
mod math;
mod colors;
mod shader;
mod area;
mod view;
mod shadergen;
mod quad;
mod text;
mod events;

pub use crate::cx::*;
pub use crate::cx_shared::*;
pub use crate::cxdrawing::*;
pub use crate::cxdrawing_shared::*;
pub use crate::cxturtle::*;
pub use crate::math::*;
pub use crate::events::*;
pub use crate::shader::*;
pub use crate::quad::*;
pub use crate::text::*;
pub use crate::colors::*;
pub use crate::elements::*;
pub use crate::animation::*;
pub use crate::area::*;
pub use crate::view::*;

pub use crate::cxturtle::Value::Computed;
pub use crate::cxturtle::Value::Fixed;
pub use crate::cxturtle::Value::Percent;
pub use crate::cxturtle::Value::Expression;
