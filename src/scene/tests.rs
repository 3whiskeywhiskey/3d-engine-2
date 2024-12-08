use super::*;
use glam::Vec4Swizzles;
use serial_test::serial;

#[test]
fn test_scene_creation() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
    let scene = Scene::new(camera);
    assert_eq!(scene.objects.len(), 0);
}

#[test]
fn test_scene_update() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
    let mut scene = Scene::new(camera);
    scene.update();
}

#[test]
fn test_scene_resize() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
    let mut scene = Scene::new(camera);
    scene.resize(800, 600);
    assert_eq!(scene.camera.aspect, 800.0 / 600.0);
}

#[test]
fn test_scene_lighting() {
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
    let mut scene = Scene::new(camera);
    
    scene.set_ambient_light(0.5);
    scene.set_directional_light(
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, -1.0),
    );
    
    assert_eq!(scene.ambient_light, Vec3::splat(0.5));
    assert_eq!(scene.directional_light, Vec3::ONE);
    assert!(scene.light_direction.is_normalized());
} 