use vulkano::
{
    instance::
    {Instance, InstanceExtensions, PhysicalDevice, ApplicationInfo, QueueFamily},
    device::
    {Device, DeviceExtensions, Features, QueuesIter},
    image::
    {SwapchainImage},
    swapchain::
    {Surface, Swapchain, SurfaceTransform, PresentMode, FullscreenExclusive},
};

use super::{VkRenderContext, VkWindowContext, SurfaceBuilder};

use std::sync::{Arc};
use std::vec::Vec;

pub fn vk_ctx_init<W>(
    app_info : ApplicationInfo,
    surface_builder : &dyn SurfaceBuilder<W>) 
    -> (Arc<VkRenderContext>, VkWindowContext<W>)
{
    let instance = create_vk_instance(&app_info);

    let (surface, window_dimensions) = 
        surface_builder.create_surface(instance.clone(), app_info.application_name.unwrap().into());

    let required_features = 
        Features 
        {
            shader_storage_image_extended_formats: true,
            ..Features::none()
        };

    let physical_device = 
        select_physical_device(&instance, &required_features);

    let queue_family = 
        select_queue_family(physical_device, surface.clone());

    let (logical_device, mut queues) = 
        logical_device_queues_init(physical_device, &required_features, queue_family);

    let queue = queues.next().unwrap();

    let render_ctx = VkRenderContext {instance, logical_device, queue};

    let (swapchain, images) = 
        swapchain_ctx_init(&render_ctx, surface.clone(), window_dimensions);

    let dynamic_state = VkWindowContext::<()>::make_dynamic_state((window_dimensions[0], window_dimensions[1]));

    let win_ctx = VkWindowContext {swapchain, surface, images, dynamic_state};

    (Arc::new(render_ctx), win_ctx)
}


fn swapchain_ctx_init<W>(render_ctx : &VkRenderContext, surface : Arc<Surface<W>>, window_dimensions : [u32 ; 2])
    -> (Arc<Swapchain<W>>, Vec<Arc<SwapchainImage<W>>>)
{
    let surface_caps = surface.capabilities(render_ctx.logical_device.physical_device()).unwrap();

    let format_colorspace = surface_caps.supported_formats[0];

    Swapchain::new(
        render_ctx.logical_device.clone(),
        surface.clone(),
        surface_caps.min_image_count,
        format_colorspace.0,
        window_dimensions,
        1,
        surface_caps.supported_usage_flags,
        &render_ctx.queue,
        SurfaceTransform::Identity,
        surface_caps.supported_composite_alpha.iter().next().unwrap(),
        PresentMode::Immediate,
        FullscreenExclusive::Default,
        true,
        format_colorspace.1
    ).unwrap()
}

fn logical_device_queues_init(physical_device : PhysicalDevice, required_features : &Features, queue_family : QueueFamily)
    -> (Arc<Device>, QueuesIter)
{
     let device_extensions =
            &DeviceExtensions
            {
                khr_storage_buffer_storage_class: true,
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };

    Device::new(
        physical_device,
        &required_features,
        device_extensions,
        [(queue_family, 0.5)].iter().cloned()
    ).expect("Vulkan Error: Failed to create device.")
}

fn create_vk_instance(app_info : &ApplicationInfo)
    -> Arc<Instance>
{
    let instance_extensions =
        &InstanceExtensions
        {
            ..vulkano_win::required_extensions()
        };

    let layers =
        vec!
        [
            "VK_LAYER_KHRONOS_validation"
        ];

    Instance::new(Some(app_info), instance_extensions, layers)
    .expect("Vulkan Error: Failed to create Vulkan instance.")
}

// Create a more involved way of selecting a physical device later...
fn select_physical_device<'a>(
    vk_instance : &'a Arc<Instance>, 
    required_features : &Features) 
    -> PhysicalDevice<'a>
{
    PhysicalDevice::enumerate(vk_instance)
        .find(|&p| 
            p.supported_features().superset_of(required_features))
        .expect("Vulkan Error: No physical device available.")
}

fn select_queue_family<W>(
    physical_device : PhysicalDevice,
    surface : Arc<Surface<W>>)
    -> QueueFamily
{
    physical_device.queue_families()
        .find(|&q|
            q.supports_graphics()
            && q.supports_compute()
            && surface.is_supported(q).unwrap_or(false))
        .expect("Vulkan Error: Could not find a supported queue family.")
}