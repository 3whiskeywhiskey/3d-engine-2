use anyhow::Result;
use ash::vk::{self, Handle};
use std::ffi::CString;
use super::{VulkanContext, Vertex, VERTICES, ViewData};
use log::{info, debug};

pub struct VrRenderer {
    device: ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    uniform_buffer: vk::Buffer,
    uniform_buffer_memory: vk::DeviceMemory,
    descriptor_set: vk::DescriptorSet,
}

impl VrRenderer {
    pub fn new(vulkan: &VulkanContext, swapchain_format: vk::Format, width: u32, height: u32) -> Result<Self> {
        unsafe {
            // Create render pass
            debug!("Creating render pass...");
            let color_attachment = vk::AttachmentDescription::builder()
                .format(swapchain_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build();

            let color_attachment_ref = vk::AttachmentReference::builder()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build();

            let subpass = vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&[color_attachment_ref])
                .build();

            let render_pass_info = vk::RenderPassCreateInfo::builder()
                .attachments(&[color_attachment])
                .subpasses(&[subpass])
                .build();

            let render_pass = vulkan.device.create_render_pass(&render_pass_info, None)?;
            debug!("Render pass created");

            // Create pipeline layout
            debug!("Creating descriptor set layout...");
            let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build();

            let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&[descriptor_set_layout_binding])
                .build();

            let descriptor_set_layout = vulkan.device.create_descriptor_set_layout(&descriptor_set_layout_info, None)?;

            debug!("Creating pipeline layout...");
            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&[descriptor_set_layout])
                .build();

            let pipeline_layout = vulkan.device.create_pipeline_layout(&pipeline_layout_info, None)?;

            // Create vertex buffer
            debug!("Creating vertex buffer...");
            let vertex_buffer_info = vk::BufferCreateInfo::builder()
                .size(std::mem::size_of_val(&VERTICES) as u64)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();

            let vertex_buffer = vulkan.device.create_buffer(&vertex_buffer_info, None)?;
            let mem_requirements = vulkan.device.get_buffer_memory_requirements(vertex_buffer);

            let memory_properties = vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device);
            let memory_type_index = find_memory_type_index(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                memory_properties,
            )?;

            let alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(memory_type_index)
                .build();

            let vertex_buffer_memory = vulkan.device.allocate_memory(&alloc_info, None)?;
            vulkan.device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)?;

            debug!("Copying vertex data...");
            let data_ptr = vulkan.device.map_memory(
                vertex_buffer_memory,
                0,
                mem_requirements.size,
                vk::MemoryMapFlags::empty(),
            )? as *mut Vertex;

            data_ptr.copy_from_nonoverlapping(VERTICES.as_ptr(), VERTICES.len());
            vulkan.device.unmap_memory(vertex_buffer_memory);
            debug!("Vertex buffer created and initialized");

            // Create command pool and buffer
            debug!("Creating command pool and buffer...");
            let command_pool_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(vulkan.queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .build();

            let command_pool = vulkan.device.create_command_pool(&command_pool_info, None)?;

            let command_buffer_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1)
                .build();

            let command_buffer = vulkan.device.allocate_command_buffers(&command_buffer_info)?[0];
            debug!("Command pool and buffer created");

            // Create graphics pipeline
            debug!("Creating graphics pipeline...");
            let graphics_pipeline = create_graphics_pipeline(
                &vulkan.device,
                render_pass,
                pipeline_layout,
                width,
                height,
            )?;
            info!("Graphics pipeline created successfully");

            // Create descriptor pool and sets
            debug!("Creating descriptor pool...");
            let pool_size = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build();

            let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&[pool_size])
                .max_sets(1)
                .build();

            let descriptor_pool = vulkan.device.create_descriptor_pool(&descriptor_pool_info, None)?;

            // Create uniform buffer for view matrices
            debug!("Creating uniform buffer...");
            let buffer_size = std::mem::size_of::<ViewData>() as u64;
            let uniform_buffer_info = vk::BufferCreateInfo::builder()
                .size(buffer_size)
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();

            let uniform_buffer = vulkan.device.create_buffer(&uniform_buffer_info, None)?;
            let mem_requirements = vulkan.device.get_buffer_memory_requirements(uniform_buffer);

            let memory_properties = vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device);
            let memory_type_index = find_memory_type_index(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                memory_properties,
            )?;

            let alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(memory_type_index)
                .build();

            let uniform_buffer_memory = vulkan.device.allocate_memory(&alloc_info, None)?;
            vulkan.device.bind_buffer_memory(uniform_buffer, uniform_buffer_memory, 0)?;

            // Allocate descriptor set
            debug!("Allocating descriptor set...");
            let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&[descriptor_set_layout])
                .build();

            let descriptor_set = vulkan.device.allocate_descriptor_sets(&descriptor_set_alloc_info)?[0];

            // Update descriptor set
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffer)
                .offset(0)
                .range(buffer_size)
                .build();

            let write_descriptor_set = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[buffer_info])
                .build();

            vulkan.device.update_descriptor_sets(&[write_descriptor_set], &[]);

            Ok(Self {
                device: vulkan.device.clone(),
                render_pass,
                pipeline_layout,
                graphics_pipeline,
                vertex_buffer,
                vertex_buffer_memory,
                command_pool,
                command_buffer,
                descriptor_pool,
                descriptor_set_layout,
                uniform_buffer,
                uniform_buffer_memory,
                descriptor_set,
            })
        }
    }

    pub fn get_render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }

    pub fn record_command_buffer(&self, framebuffer: vk::Framebuffer, width: u32, height: u32) -> Result<()> {
        unsafe {
            debug!("Beginning command buffer recording...");
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();

            self.device.begin_command_buffer(self.command_buffer, &begin_info)?;

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width, height },
            };

            debug!("Beginning render pass...");
            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(framebuffer)
                .render_area(render_area)
                .clear_values(&clear_values)
                .build();

            self.device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            debug!("Binding pipeline and vertex buffer...");
            self.device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            debug!("Binding descriptor set...");
            self.device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );

            self.device.cmd_bind_vertex_buffers(
                self.command_buffer,
                0,
                &[self.vertex_buffer],
                &[0],
            );

            debug!("Recording draw command...");
            self.device.cmd_draw(
                self.command_buffer,
                VERTICES.len() as u32,
                1,
                0,
                0,
            );

            self.device.cmd_end_render_pass(self.command_buffer);
            self.device.end_command_buffer(self.command_buffer)?;
            debug!("Command buffer recording completed");

            Ok(())
        }
    }

    pub fn submit_commands(&self, queue: vk::Queue) -> Result<()> {
        unsafe {
            debug!("Submitting command buffer...");
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&[self.command_buffer])
                .build();

            self.device.queue_submit(queue, &[submit_info], vk::Fence::null())?;
            self.device.queue_wait_idle(queue)?;
            debug!("Command buffer submitted and executed");

            Ok(())
        }
    }

    pub fn update_view_matrices(&self, view_data: &ViewData) -> Result<()> {
        unsafe {
            debug!("Updating view matrices...");
            let data_ptr = self.device.map_memory(
                self.uniform_buffer_memory,
                0,
                std::mem::size_of::<ViewData>() as u64,
                vk::MemoryMapFlags::empty(),
            )? as *mut ViewData;

            data_ptr.write(*view_data);
            self.device.unmap_memory(self.uniform_buffer_memory);
            debug!("View matrices updated");

            Ok(())
        }
    }
}

impl Drop for VrRenderer {
    fn drop(&mut self) {
        info!("Cleaning up renderer resources");
        unsafe {
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_buffer(self.uniform_buffer, None);
            self.device.free_memory(self.uniform_buffer_memory, None);
        }
    }
}

fn find_memory_type_index(
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
) -> Result<u32> {
    for i in 0..memory_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && memory_properties.memory_types[i as usize]
                .property_flags
                .contains(properties)
        {
            return Ok(i);
        }
    }
    Err(anyhow::anyhow!("Failed to find suitable memory type"))
}

fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    width: u32,
    height: u32,
) -> Result<vk::Pipeline> {
    debug!("Loading shader code...");
    let vert_shader_code = include_bytes!("../../shaders/triangle.vert.spv");
    let frag_shader_code = include_bytes!("../../shaders/triangle.frag.spv");

    debug!("Creating shader modules...");
    let vertex_shader_module = create_shader_module(device, vert_shader_code)?;
    let fragment_shader_module = create_shader_module(device, frag_shader_code)?;

    let main_function_name = CString::new("main").unwrap();

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_shader_module)
            .name(&main_function_name)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_shader_module)
            .name(&main_function_name)
            .build(),
    ];

    debug!("Setting up pipeline state...");
    let binding_description = Vertex::get_binding_description();
    let attribute_descriptions = Vertex::get_attribute_descriptions();

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&[binding_description])
        .vertex_attribute_descriptions(&attribute_descriptions)
        .build();

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
        .build();

    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: width as f32,
        height: height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };

    let scissor = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk::Extent2D { width, height },
    };

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&[viewport])
        .scissors(&[scissor])
        .build();

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .build();

    let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .build();

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false)
        .build();

    let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&[color_blend_attachment])
        .build();

    debug!("Creating graphics pipeline...");
    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();

    let pipeline = unsafe {
        let pipelines = device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_info],
            None,
        ).map_err(|e| anyhow::anyhow!("Failed to create graphics pipeline: {:?}", e))?;
        pipelines[0]
    };

    debug!("Cleaning up shader modules...");
    unsafe {
        device.destroy_shader_module(vertex_shader_module, None);
        device.destroy_shader_module(fragment_shader_module, None);
    }

    Ok(pipeline)
}

fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule> {
    let code = unsafe { std::slice::from_raw_parts(
        code.as_ptr() as *const u32,
        code.len() / 4,
    )};

    let create_info = vk::ShaderModuleCreateInfo::builder()
        .code(code)
        .build();

    unsafe {
        Ok(device.create_shader_module(&create_info, None)?)
    }
} 