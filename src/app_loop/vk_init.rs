use vulkano::
{
    instance::
    {Instance},
    device::
    {Device, Queue},
    image::
    {SwapchainImage},
    swapchain::
    {Surface, Swapchain},
    command_buffer::
    {DynamicState},
    framebuffer::
    {Framebuffer, FramebufferAbstract, RenderPassAbstract},
    pipeline::viewport::Viewport
};

pub mod vk_renderer;
pub mod vk_window;

use std::sync::Arc;

pub struct VkRenderContext
{
    pub instance            : Arc<Instance>,
    pub logical_device      : Arc<Device>,
    pub queue               : Arc<Queue>
}

pub struct VkWindowContext<W>
{
    pub swapchain           : Arc<Swapchain<W>>,
    pub surface             : Arc<Surface<W>>,
    pub images              : Vec<Arc<SwapchainImage<W>>>,
    pub dynamic_state       : DynamicState,
}

pub type FramebuffersGroup = Vec<Arc<dyn FramebufferAbstract + Send + Sync>>;
type RenderpassArced = Arc<dyn RenderPassAbstract + Send + Sync>;
impl<W> VkWindowContext<W>
    where W : Send + Sync + 'static
{
    pub fn update_swapchain(
        &mut self, 
        dimensions : [u32 ; 2])
    {
        // First updates the variables of the struct
        let res = match self.swapchain.recreate_with_dimensions(dimensions) {
            Ok(r) => r,
            Err(err) => panic!("{:?}", err)
        };
        self.swapchain = res.0;
        self.images = res.1;
    }

    pub fn update_with_res_scale(
        &mut self,
        res_scale : [f32 ; 2])
    {
        self.update_swapchain(self.scaled_dims(res_scale));
    }
    
    pub fn framebuffers_init(
        &mut self,
        render_pass: RenderpassArced)
        // in_attach : Arc<AttachmentImage>)
        -> FramebuffersGroup
        where W : Send + Sync + 'static
    {

        self.images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>()
    }



    pub fn aspect_ratio(&self)
        -> f32
    {
        let dims = self.swapchain.dimensions();
        dims[1] as f32 / dims[0] as f32
    }

    pub fn dims(&self)
        -> [u32 ; 2]
    {
        self.swapchain.dimensions()
    }

    pub fn scaled_dims(&self, render_scale : [f32 ; 2])
        -> [u32 ; 2]
    {
        let win_res = self.dims();
        [(win_res[0] as f32 * render_scale[0]) as u32, (win_res[1] as f32 * render_scale[1]) as u32]
    }

    pub fn make_dynamic_state(dims : (u32, u32))
        -> DynamicState
    {
        DynamicState
        {
            viewports : 
                Some(vec!(Viewport 
                {
                    origin: [0.0, 0.0],
                    dimensions: [dims.0 as f32, dims.1 as f32],
                    depth_range: 0.0 .. 1.0,
                })),
            ..DynamicState::none()
        }
    }

    pub fn update_dynamic_state_with_dims(&mut self, render_dims : [u32 ; 2])
    {
        let win_dims = self.dims();
        let clamped_dims = (render_dims[0].min(win_dims[0]), render_dims[1].min(win_dims[1]));
        self.dynamic_state.viewports = Some(vec!(Viewport {
            origin: [0.0, 0.0],
            dimensions: [clamped_dims.0 as f32, clamped_dims.1 as f32],
            depth_range: 0.0 .. 1.0,
        }));
    }

    pub fn update_dynamic_state_with_scale(&mut self, render_scale : [f32 ; 2])
    {
        self.update_dynamic_state_with_dims(self.scaled_dims(render_scale))
    }
}

pub trait SurfaceBuilder<W>
{
    fn create_surface(&self, instance : Arc<Instance>, title : String)
        -> (Arc<Surface<W>>, [u32 ; 2]);
}