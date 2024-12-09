use openxr as xr;
use anyhow::Result;

use super::math::ViewProjection;

#[derive(Debug, Clone)]
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

    pub fn initialize_session(
        &mut self,
        session: xr::Session<xr::Vulkan>,
        frame_waiter: xr::FrameWaiter,
        frame_stream: xr::FrameStream<xr::Vulkan>,
        views: Vec<xr::ViewConfigurationView>,
    ) {
        self.session = Some(session);
        self.frame_waiter = Some(frame_waiter);
        self.frame_stream = Some(frame_stream);
        self.views = Some(views);
    }

    pub fn initialize_resources(
        &mut self,
        swapchain: xr::Swapchain<xr::Vulkan>,
        stage: xr::Space,
    ) {
        self.swapchain = Some(swapchain);
        self.stage = Some(stage);
    }

    pub fn get_session(&self) -> Option<&xr::Session<xr::Vulkan>> {
        self.session.as_ref()
    }

    pub fn wait_frame(&mut self) -> Result<xr::FrameState> {
        if let Some(frame_waiter) = &mut self.frame_waiter {
            Ok(frame_waiter.wait()?)
        } else {
            Err(anyhow::anyhow!("Frame waiter not initialized"))
        }
    }

    pub fn begin_frame(&mut self) -> Result<xr::FrameState> {
        if let (Some(frame_waiter), Some(frame_stream)) = (&mut self.frame_waiter, &mut self.frame_stream) {
            // Wait for the next frame
            let frame_state = frame_waiter.wait()?;
            
            // Begin the frame
            frame_stream.begin().map_err(|e| anyhow::anyhow!("Failed to begin frame: {}", e))?;
            
            Ok(frame_state)
        } else {
            Err(anyhow::anyhow!("Frame waiter or stream not initialized"))
        }
    }

    pub fn acquire_swapchain_image(&mut self) -> Result<u32> {
        if let Some(swapchain) = &mut self.swapchain {
            let image_index = swapchain.acquire_image()?;
            log::info!("Acquired swapchain image {}", image_index);
            // Use a shorter timeout to avoid blocking too long
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

    pub fn submit_frame(
        &mut self,
        frame_state: xr::FrameState,
        view_projections: &[ViewProjection],
        width: u32,
        height: u32,
    ) -> Result<()> {
        let swapchain = self.swapchain.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Swapchain not initialized"))?;

        // Create composition layer views
        let mut views = Vec::with_capacity(view_projections.len());
        for (i, view_proj) in view_projections.iter().enumerate() {
            let view = xr::CompositionLayerProjectionView::new()
                .pose(view_proj.pose)
                .fov(view_proj.fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(swapchain)
                        .image_array_index(i as u32)
                        .image_rect(xr::Rect2Di {
                            offset: xr::Offset2Di { x: 0, y: 0 },
                            extent: xr::Extent2Di {
                                width: width as i32,
                                height: height as i32,
                            },
                        }),
                );
            views.push(view);
        }

        // End frame with composition layers
        if let Some(frame_stream) = &mut self.frame_stream {
            if let Some(stage) = &self.stage {
                let projection_layer = xr::CompositionLayerProjection::new().space(stage).views(&views);
                frame_stream.end(
                    frame_state.predicted_display_time,
                    xr::EnvironmentBlendMode::OPAQUE,
                    &[&projection_layer],
                )?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Stage not initialized"))
            }
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

    pub fn get_swapchain(&self) -> Option<&xr::Swapchain<xr::Vulkan>> {
        self.swapchain.as_ref()
    }

    pub fn get_frame_stream_mut(&mut self) -> Option<&mut xr::FrameStream<xr::Vulkan>> {
        self.frame_stream.as_mut()
    }

    pub fn get_frame_stream(&self) -> Option<&xr::FrameStream<xr::Vulkan>> {
        self.frame_stream.as_ref()
    }

    pub fn get_stage(&self) -> Option<&xr::Space> {
        self.stage.as_ref()
    }

    pub fn submit_frame_with_projection(
        &mut self,
        frame_state: &xr::FrameState,
        views: &[xr::View],
    ) -> Result<()> {
        // Get view configuration for dimensions
        let view_config = self.views.as_ref()
            .ok_or_else(|| anyhow::anyhow!("View configuration not initialized"))?;
        let view_dimensions = &view_config[0]; // Both eyes use same dimensions

        // Get mutable reference to swapchain for image operations
        let swapchain = self.swapchain.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Swapchain not initialized"))?;
        
        // Acquire and wait for swapchain image
        let _image_index = swapchain.acquire_image()?;
        swapchain.wait_image(xr::Duration::from_nanos(100_000_000))?;

        // Create projection views
        let projection_views: Vec<_> = views.iter()
            .map(|view| xr::CompositionLayerProjectionView::new()
                .pose(view.pose)
                .fov(view.fov)
                .sub_image(xr::SwapchainSubImage::new()
                    .swapchain(swapchain)
                    .image_array_index(0)
                    .image_rect(xr::Rect2Di {
                        offset: xr::Offset2Di { x: 0, y: 0 },
                        extent: xr::Extent2Di {
                            width: view_dimensions.recommended_image_rect_width as i32,
                            height: view_dimensions.recommended_image_rect_height as i32,
                        },
                    })))
            .collect();

        // Get stage space
        let stage = self.stage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Stage space not initialized"))?;

        // Create projection layer
        let projection_layer = xr::CompositionLayerProjection::new()
            .space(stage)
            .views(&projection_views);

        // Begin frame stream and submit
        if let Some(frame_stream) = &mut self.frame_stream {
            frame_stream.begin()?;
            log::info!("Successfully began frame stream");

            frame_stream.end(
                frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[&projection_layer],
            )?;
            log::info!("Successfully submitted frame");

            // Release the swapchain image after submission
            swapchain.release_image()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Frame stream not initialized"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openxr as xr;

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

    #[test]
    fn test_view_projection_creation() {
        // Create mock views
        let views = vec![
            xr::View {
                pose: xr::Posef {
                    orientation: xr::Quaternionf {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                    position: xr::Vector3f {
                        x: -0.032, // IPD offset for left eye
                        y: 0.0,
                        z: 0.0,
                    },
                },
                fov: xr::Fovf {
                    angle_left: -1.0,
                    angle_right: 1.0,
                    angle_up: 1.0,
                    angle_down: -1.0,
                },
            },
            xr::View {
                pose: xr::Posef {
                    orientation: xr::Quaternionf {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                    position: xr::Vector3f {
                        x: 0.032, // IPD offset for right eye
                        y: 0.0,
                        z: 0.0,
                    },
                },
                fov: xr::Fovf {
                    angle_left: -1.0,
                    angle_right: 1.0,
                    angle_up: 1.0,
                    angle_down: -1.0,
                },
            },
        ];

        // Test view projection creation
        let view_projections: Vec<ViewProjection> = views.iter()
            .map(|view| ViewProjection::from_xr_view(view, 0.1))
            .collect();

        // Verify view projections
        assert_eq!(view_projections.len(), 2);
        
        // Check that the view matrices are different for each eye
        assert_ne!(view_projections[0].view, view_projections[1].view);
        
        // Check that the projection matrices are symmetric
        assert_eq!(view_projections[0].projection, view_projections[1].projection);
    }
} 