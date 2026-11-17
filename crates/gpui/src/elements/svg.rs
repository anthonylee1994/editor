use std::collections::hash_map::DefaultHasher;
use std::f32::consts::TAU;
use std::fs;
use std::hash::Hash as _;
use std::hash::Hasher as _;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use gpui_util::ResultExt as _;

use crate::App;
use crate::Asset;
use crate::Bounds;
use crate::Element;
use crate::GlobalElementId;
use crate::Hitbox;
use crate::InteractiveElement;
use crate::Interactivity;
use crate::IntoElement;
use crate::LayoutId;
use crate::Pixels;
use crate::Point;
use crate::Radians;
use crate::SharedString;
use crate::Size;
use crate::SpriteAnimation;
use crate::StyleRefinement;
use crate::Styled;
use crate::TransformationMatrix;
use crate::Window;
use crate::point;
use crate::px;
use crate::radians;
use crate::size;

/// An SVG element.
pub struct Svg {
    interactivity: Interactivity,
    transformation: Option<Transformation>,
    rotation_period: Option<Duration>,
    path: Option<SharedString>,
    external_path: Option<SharedString>,
    data: Option<Arc<[u8]>>,
    data_path: Option<SharedString>,
}

/// Create a new SVG element.
pub fn svg() -> Svg {
    Svg {
        interactivity: Interactivity::default(),
        transformation: None,
        rotation_period: None,
        path: None,
        external_path: None,
        data: None,
        data_path: None,
    }
}

impl Svg {
    /// Set the path to the SVG file for this element.
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the path to the SVG file for this element.
    pub fn external_path(mut self, path: impl Into<SharedString>) -> Self {
        self.external_path = Some(path.into());
        self
    }

    /// Set the raw SVG data for this element.
    /// The SVG will be rendered directly from the provided bytes.
    pub fn data(mut self, data: &[u8]) -> Self {
        // Generate a unique deterministic path based on the data hash for caching
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        let path = SharedString::from(format!("__binary_svg__{hash}"));
        self.data = Some(Arc::from(data));
        self.data_path = Some(path);
        self
    }

    /// Transform the SVG element with the given transformation.
    /// Note that this won't effect the hitbox or layout of the element, only the rendering.
    pub const fn with_transformation(mut self, transformation: Transformation) -> Self {
        self.transformation = Some(transformation);
        self
    }

    /// Rotate the SVG clockwise continuously, one full turn per `period`, starting from
    /// [`Self::with_transformation`] and phase-locked to the app's shared animation clock.
    ///
    /// The SVG is painted once; the window turns it in the scene it already has and
    /// presents that again, so no view re-renders to advance it.
    pub const fn with_rotation_period(mut self, period: Duration) -> Self {
        self.rotation_period = Some(period);
        self
    }
}

impl Element for Svg {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<crate::ElementId> {
        self.interactivity.element_id.clone()
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self
            .interactivity
            .request_layout(global_id, window, cx, |style, window, cx| window.request_layout(style, None, cx));
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Hitbox> {
        self.interactivity.prepaint(global_id, bounds, bounds.size, window, cx, |_, _, hitbox, _, _| hitbox)
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) where
        Self: Sized,
    {
        self.interactivity.paint(global_id, bounds, hitbox.as_ref(), window, cx, |style, window, cx| {
            let center = bounds.center();
            let scale_factor = window.scale_factor();
            let base = self.transformation.unwrap_or_default();
            let transformation = base.into_matrix(center, scale_factor);
            let animation = self
                .rotation_period
                .filter(|period| !period.is_zero())
                .map(|period| SpriteAnimation::new(base, period, center, scale_factor));

            if let Some((data, path)) = self.data.as_ref().zip(self.data_path.as_ref()) {
                if let Some(color) = style.text.color {
                    window
                        .paint_svg_sprite(bounds, path.clone(), Some(&**data), transformation, animation, color, cx)
                        .log_err();
                }
            } else if let Some((path, color)) = self.external_path.as_ref().zip(style.text.color) {
                let Some(bytes) = window.use_asset::<SvgAsset>(path, cx).and_then(gpui_util::ResultExt::log_err) else {
                    return;
                };

                window
                    .paint_svg_sprite(bounds, path.clone(), Some(&bytes), transformation, animation, color, cx)
                    .log_err();
            } else if let Some((path, color)) = self.path.as_ref().zip(style.text.color) {
                window.paint_svg_sprite(bounds, path.clone(), None, transformation, animation, color, cx).log_err();
            }
        });
    }
}

impl IntoElement for Svg {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for Svg {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for Svg {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

/// A transformation to apply to an SVG element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transformation {
    scale: Size<f32>,
    translate: Point<Pixels>,
    rotate: Radians,
}

impl Default for Transformation {
    fn default() -> Self {
        Self { scale: size(1.0, 1.0), translate: point(px(0.0), px(0.0)), rotate: radians(0.0) }
    }
}

impl Transformation {
    /// Create a new Transformation with the specified scale along each axis.
    pub const fn scale(scale: Size<f32>) -> Self {
        Self { scale, translate: point(px(0.0), px(0.0)), rotate: radians(0.0) }
    }

    /// Create a new Transformation with the specified translation.
    pub const fn translate(translate: Point<Pixels>) -> Self {
        Self { scale: size(1.0, 1.0), translate, rotate: radians(0.0) }
    }

    /// Create a new Transformation with the specified rotation in radians.
    pub fn rotate(rotate: impl Into<Radians>) -> Self {
        let rotate = rotate.into();
        Self { scale: size(1.0, 1.0), translate: point(px(0.0), px(0.0)), rotate }
    }

    /// This transformation turned clockwise by as much of a full turn as `elapsed` is into `period`.
    pub(crate) fn turned(self, period: Duration, elapsed: Duration) -> Self {
        // Reduce modulo the period before f32 conversion, which loses sub-second precision at scale.
        let turn =
            Duration::from_nanos((elapsed.as_nanos() % period.as_nanos()) as u64).as_secs_f32() / period.as_secs_f32();
        Self { rotate: radians(self.rotate.0 + TAU * turn), ..self }
    }

    pub(crate) fn into_matrix(self, center: Point<Pixels>, scale_factor: f32) -> TransformationMatrix {
        //Note: if you read this as a sequence of matrix multiplications, start from the bottom
        TransformationMatrix::unit()
            .translate(center.scale(scale_factor) + self.translate.scale(scale_factor))
            .rotate(self.rotate)
            .scale(self.scale)
            .translate(center.scale(-scale_factor))
    }
}

struct SvgAsset(!);

impl Asset for SvgAsset {
    type Source = SharedString;
    type Output = Result<Arc<[u8]>, Arc<io::Error>>;

    #[allow(clippy::manual_async_fn, reason = "an `async fn` would capture `cx` and not be `'static`")]
    fn load(source: Self::Source, _cx: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            let bytes = fs::read(Path::new(source.as_ref())).map_err(Arc::new)?;
            let bytes = Arc::from(bytes);
            Ok(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;
    use std::f32::consts::PI;
    use std::time::Duration;

    use super::Transformation;
    use crate::radians;

    #[test]
    fn turned_adds_the_elapsed_fraction_of_a_turn() {
        let period = Duration::from_secs(2);
        let base = Transformation::rotate(radians(FRAC_PI_2));
        let rotation = |elapsed_ms| base.turned(period, Duration::from_millis(elapsed_ms)).rotate.0;

        assert!((rotation(0) - FRAC_PI_2).abs() < 1e-5);
        assert!((rotation(500) - PI).abs() < 1e-5);
        // Whole periods don't accumulate, however long the clock has run.
        assert!((rotation(3_600_000_500) - PI).abs() < 1e-5);
    }
}
