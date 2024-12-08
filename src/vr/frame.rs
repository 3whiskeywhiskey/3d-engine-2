use openxr as xr;
use anyhow::Result;

use super::math::ViewProjection;

#[derive(Debug)]
pub struct FrameResources {
    pub frame_state: xr::FrameState,
    pub view_projections: Vec<ViewProjection>,
}

pub struct FrameManager {
    frame_waiter: Option<xr::FrameWaiter>,
    frame_stream: Option<xr::FrameStream<xr::Vulkan>>,
    swapchain: Option<xr::Swapchain<xr::Vulkan>>,
    stage: Option<xr::Space>,
    session: Option<xr::Session<xr::Vulkan>>,
    views: Option<Vec<xr::ViewConfigurationView>>,
}

impl FrameManager {
    pub fn new() -> Self {
        Self {
            frame_waiter: None,
            frame_stream: None,
            swapchain: None,
            stage: None,
            session: None,
            views: None,
        }
    }

    pub fn initialize(
        &mut self,
        session: xr::Session<xr::Vulkan>,
        frame_waiter: xr::FrameWaiter,
        frame_stream: xr::FrameStream<xr::Vulkan>,
        swapchain: xr::Swapchain<xr::Vulkan>,
        stage: xr::Space,
        views: Vec<xr::ViewConfigurationView>,
    ) {
        self.session = Some(session);
        self.frame_waiter = Some(frame_waiter);
        self.frame_stream = Some(frame_stream);
        self.swapchain = Some(swapchain);
        self.stage = Some(stage);
        self.views = Some(views);
    }

    pub fn get_session(&self) -> Option<&xr::Session<xr::Vulkan>> {
        self.session.as_ref()
    }

    pub fn begin_frame(&mut self) -> Result<xr::FrameState> {
        if let (Some(frame_waiter), Some(frame_stream)) = (&mut self.frame_waiter, &mut self.frame_stream) {
            frame_waiter.wait()?;
            let frame_state = xr::FrameState {
                predicted_display_time: xr::Time::from_nanos(0),  // We'll get the actual time from the runtime later
                predicted_display_period: xr::Duration::from_nanos(0),
                should_render: true,  // We'll assume we should always render for now
            };
            frame_stream.begin().map_err(|e| anyhow::anyhow!("Failed to begin frame: {:?}", e))?;
            Ok(frame_state)
        } else {
            Err(anyhow::anyhow!("Frame waiter or stream not initialized"))
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Result<u32> {
        if let Some(swapchain) = &mut self.swapchain {
            let image_index = swapchain.acquire_image()?;
            swapchain.wait_image(xr::Duration::from_nanos(100_000_000))?;
            Ok(image_index)
        } else {
            Err(anyhow::anyhow!("Swapchain not initialized"))
        }
    }

    pub fn release_swapchain_image(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.release_image()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Swapchain not initialized"))
        }
    }

    pub fn end_frame(&mut self, frame_state: xr::FrameState, views: &[xr::CompositionLayerProjectionView<xr::Vulkan>]) -> Result<()> {
        if let (Some(frame_stream), Some(stage)) = (&mut self.frame_stream, &self.stage) {
            let projection_layer = xr::CompositionLayerProjection::new().space(stage).views(views);
            frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[&projection_layer],
            )?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Frame stream not initialized"))
        }
    }

    pub fn get_views(&self, frame_state: &xr::FrameState) -> Result<Vec<xr::View>> {
        if let (Some(session), Some(stage)) = (&self.session, &self.stage) {
            let (_, views) = session.locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                frame_state.predicted_display_time,
                stage,
            )?;
            Ok(views)
        } else {
            Err(anyhow::anyhow!("Session or stage not initialized"))
        }
    }

    pub fn get_view_projections(&self, frame_state: &xr::FrameState) -> Result<Vec<ViewProjection>> {
        let views = self.get_views(frame_state)?;
        
        let mut view_projections = Vec::new();
        for view in views {
            view_projections.push(ViewProjection::from_xr_view(&view, 0.001));  // Near plane = 0.001
        }

        Ok(view_projections)
    }

    pub fn get_swapchain_image_layout(&self) -> Option<(u32, u32)> {
        self.views.as_ref().map(|views| {
            let view = &views[0];  // Both eyes use the same resolution
            (
                view.recommended_image_rect_width,
                view.recommended_image_rect_height,
            )
        })
    }

    pub fn take_session_components(self) -> Option<(
        xr::Session<xr::Vulkan>,
        xr::FrameWaiter,
        xr::FrameStream<xr::Vulkan>,
        xr::Swapchain<xr::Vulkan>,
        xr::Space,
        Vec<xr::ViewConfigurationView>,
    )> {
        match (
            self.session,
            self.frame_waiter,
            self.frame_stream,
            self.swapchain,
            self.stage,
            self.views,
        ) {
            (
                Some(session),
                Some(frame_waiter),
                Some(frame_stream),
                Some(swapchain),
                Some(stage),
                Some(views),
            ) => Some((session, frame_waiter, frame_stream, swapchain, stage, views)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_manager_new() {
        let frame_manager = FrameManager::new();
        assert!(frame_manager.frame_waiter.is_none());
        assert!(frame_manager.frame_stream.is_none());
        assert!(frame_manager.swapchain.is_none());
        assert!(frame_manager.stage.is_none());
        assert!(frame_manager.session.is_none());
        assert!(frame_manager.views.is_none());
    }
} 