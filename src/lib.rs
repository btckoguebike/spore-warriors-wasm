use molecule::prelude::Entity;
use rand::{rngs::SmallRng, SeedableRng};
use spore_warriors_core::map::MapSkeleton;
use spore_warriors_core::wrappings::Warrior;
use spore_warriors_generated as generated;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

#[wasm_bindgen]
pub fn greet(raw_resource_pool: &[u8]) {
    log(format!("greet: {}", raw_resource_pool.len()).as_str());
    let resource_pool = match generated::ResourcePool::from_compatible_slice(raw_resource_pool) {
        Ok(pool) => pool,
        Err(err) => {
            error(&err.to_string());
            return;
        }
    };
    let warrier = resource_pool.warrior_pool().get_unchecked(0);
    let mut rng = SmallRng::seed_from_u64(10086);

    let player = Warrior::randomized(&resource_pool, warrier, &mut rng).unwrap();
    let map = MapSkeleton::randomized(&resource_pool, player, (1, 0).into(), &mut rng).unwrap();
    log(format!("[map] = {map:?}").as_str());
    log(format!("[avaliable_range] = {:?}", map.movable_range()).as_str());
    log(format!("[node] = {:?}", map.peak_upcoming_movment((1, 1).into())).as_str());
}
