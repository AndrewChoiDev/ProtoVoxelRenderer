use winit::
{
    event_loop::
    {EventLoop},
    window::
    {Window, WindowBuilder}
};
use std::sync::Arc;
use vulkano::swapchain::Surface;
use vulkano::instance::Instance;
use super::SurfaceBuilder;



impl SurfaceBuilder<Window> for EventLoop<()>
{
    // Returns surface and its dimensions
    fn create_surface(&self, instance : Arc<Instance>, title : String)
        -> (Arc<Surface<Window>>, [u32 ; 2])
    {
        // let monitor_handle = self.primary_monitor();


        let surface = vulkano_win::create_vk_surface(
            WindowBuilder::new()
            .with_title(title)
            // .with_max_inner_size(monitor_handle.size())
            .with_min_inner_size(winit::dpi::PhysicalSize {width: 12, height: 12})
            .with_inner_size(winit::dpi::PhysicalSize {width: 1200, height: 900})
            .build(self).unwrap(),
            instance.clone()
        ).unwrap();

        (surface.clone(), surface.window().inner_size().into())
    }
}