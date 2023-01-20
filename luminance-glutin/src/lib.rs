//! The [glutin](https://crates.io/crates/glutin) platform crate for [luminance](https://crates.io/crates/luminance).

#![deny(missing_docs)]

use gl;
use glutin::config::{Api, ConfigTemplateBuilder};
use glutin::context::{
  ContextApi, ContextAttributesBuilder, GlProfile, PossiblyCurrentContext, Version,
};
use glutin::display::GetGlDisplay;
use glutin::prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor};
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use luminance::context::GraphicsContext;
use luminance::framebuffer::{Framebuffer, FramebufferError};
use luminance::texture::Dim2;
pub use luminance_gl::gl33::StateQueryError;
use luminance_gl::GL33;
use raw_window_handle::HasRawWindowHandle;
use std::error;
use std::ffi::CString;
use std::fmt;
use std::num::NonZeroU32;
use std::os::raw::c_void;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

/// Error that might occur when creating a Glutin surface.
#[derive(Debug)]
pub enum GlutinError {
  /// OpenGL context error.
  ContextError(glutin::error::Error),
  /// Error from [`glutin_winit::DisplayBuilder`].
  CreateWindowError(Box<dyn std::error::Error>),
  /// [`glutin_winit::DisplayBuilder`] did not return a window
  NoWindowError,
  /// Graphics state error that might occur when querying the initial state.
  GraphicsStateError(StateQueryError),
}

impl fmt::Display for GlutinError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      GlutinError::ContextError(ref e) => write!(f, "Glutin OpenGL context creation error: {}", e),
      GlutinError::CreateWindowError(ref e) => write!(f, "Window creation error: {}", e),
      GlutinError::NoWindowError => f.write_str("Display builder did not return a window"),
      GlutinError::GraphicsStateError(ref e) => {
        write!(f, "OpenGL graphics state initialization error: {}", e)
      }
    }
  }
}

impl error::Error for GlutinError {
  fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    match self {
      GlutinError::ContextError(e) => Some(e),
      GlutinError::CreateWindowError(ref e) => Some(&**e),
      GlutinError::NoWindowError => None,
      GlutinError::GraphicsStateError(e) => Some(e),
    }
  }
}

impl From<glutin::error::Error> for GlutinError {
  fn from(e: glutin::error::Error) -> Self {
    GlutinError::ContextError(e)
  }
}

impl From<StateQueryError> for GlutinError {
  fn from(e: StateQueryError) -> Self {
    GlutinError::GraphicsStateError(e)
  }
}

/// The Glutin surface.
///
/// You want to create such an object in order to use any [luminance] construct.
///
/// [luminance]: https://crates.io/crates/luminance
pub struct GlutinSurface {
  /// The windowed context.
  pub ctx: PossiblyCurrentContext,
  /// The window rendering surface
  surface: Surface<WindowSurface>,
  /// The window
  window: Window,
  /// OpenGL 3.3 state.
  gl: GL33,
}

unsafe impl GraphicsContext for GlutinSurface {
  type Backend = GL33;

  fn backend(&mut self) -> &mut Self::Backend {
    &mut self.gl
  }
}

impl GlutinSurface {
  /// Create a new [`GlutinSurface`] from scratch.
  pub fn new_gl33(
    window_builder: WindowBuilder,
    samples: u8,
  ) -> Result<(Self, EventLoop<()>), GlutinError> {
    let event_loop = EventLoop::new();
    let ctb = ConfigTemplateBuilder::new().with_multisampling(samples);
    let surface = Self::new_gl33_windowed_with_builders(
      &event_loop,
      window_builder,
      ContextAttributesBuilder::new(),
      SurfaceAttributesBuilder::new(),
      ctb,
      |mut cfgs| cfgs.next().unwrap(),
    )?;

    Ok((surface, event_loop))
  }

  /// Create a new [`GlutinSurface`] by passing in the builders.
  ///
  /// This is the most flexible but least hand-holding way of creating a [`GlutinSurface`].
  ///
  /// A few overrides are put in place for what luminance requires. Specifically, the [`ConfigTemplateBuilder`] is
  /// edited to specify the OpenGL API and disable single buffer mode, and the [`ContextAttributesBuilder`]
  /// is edited to request an OpenGL 3.3 core profile.
  ///
  pub fn new_gl33_windowed_with_builders<EL, Picker>(
    event_loop: &EventLoop<EL>,
    window_builder: WindowBuilder,
    context_attributes: ContextAttributesBuilder,
    surface_attributes: SurfaceAttributesBuilder<WindowSurface>,
    config_template: ConfigTemplateBuilder,
    config_picker: Picker,
  ) -> Result<Self, GlutinError>
  where
    Picker: FnOnce(Box<dyn Iterator<Item = glutin::config::Config> + '_>) -> glutin::config::Config,
  {
    let config_template = config_template
      .with_api(Api::OPENGL)
      .with_single_buffering(false);
    let surface_attributes = surface_attributes.with_single_buffer(false);

    let (window, gl_config) = DisplayBuilder::new()
      .with_preference(glutin_winit::ApiPrefence::FallbackEgl)
      .with_window_builder(Some(window_builder))
      .build(event_loop, config_template, config_picker)
      .map_err(|e| GlutinError::CreateWindowError(e))?;
    let window = window.ok_or(GlutinError::NoWindowError)?;

    let gl_display = gl_config.display();

    let context_attributes = context_attributes
      .with_profile(GlProfile::Core)
      .with_context_api(ContextApi::OpenGl(Some(Version { major: 3, minor: 3 })))
      .build(Some(window.raw_window_handle()));
    let ctx = unsafe { gl_display.create_context(&gl_config, &context_attributes) }?;

    let surface = unsafe {
      let size = window.inner_size();
      gl_display.create_window_surface(
        &gl_config,
        &surface_attributes.build(
          window.raw_window_handle(),
          NonZeroU32::new(size.width).unwrap(),
          NonZeroU32::new(size.height).unwrap(),
        ),
      )?
    };

    let ctx = ctx.make_current(&surface)?;

    // init OpenGL
    gl::load_with(|s| gl_display.get_proc_address(&CString::new(s).unwrap()) as *const c_void);

    window.set_visible(true);

    let gl = GL33::new().map_err(GlutinError::GraphicsStateError)?;
    let surface = GlutinSurface {
      ctx,
      window,
      surface,
      gl,
    };
    Ok(surface)
  }

  /// Get the underlying size (in physical pixels) of the surface.
  ///
  /// This is equivalent to getting the inner size of the windowed context and converting it to
  /// a physical size by using the HiDPI factor of the windowed context.
  pub fn size(&self) -> [u32; 2] {
    let size = self.window.inner_size();
    [size.width, size.height]
  }

  /// Notify the context of a window resize.
  ///
  /// Should be called in response to window resize events.
  pub fn resize(&self) {
    let size = self.window.inner_size();
    self.surface.resize(
      &self.ctx,
      NonZeroU32::new(size.width).unwrap(),
      NonZeroU32::new(size.height).unwrap(),
    );
  }

  /// Get access to the back buffer.
  pub fn back_buffer(&mut self) -> Result<Framebuffer<GL33, Dim2, (), ()>, FramebufferError> {
    Framebuffer::back_buffer(self, self.size())
  }

  /// Swap the back and front buffers.
  pub fn swap_buffers(&mut self) -> Result<(), glutin::error::Error> {
    self.surface.swap_buffers(&self.ctx)
  }

  /// Gets the underlying window
  pub fn window(&self) -> &Window {
    &self.window
  }

  /// Sets the swap interval for the surface.
  pub fn set_swap_interval(
    &self,
    interval: glutin::surface::SwapInterval,
  ) -> Result<(), glutin::error::Error> {
    self.surface.set_swap_interval(&self.ctx, interval)
  }
}
