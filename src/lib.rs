use std::sync::Mutex;

use lazy_static::lazy_static;
use serde_wasm_bindgen::Error;
use spore_warriors_core::battle::pve::MapBattlePVE;
use spore_warriors_core::battle::traits::{IterationInput, Selection, SimplePVE};
use spore_warriors_core::contexts::{WarriorContext, WarriorDeckContext};
use spore_warriors_core::game::Game;
use spore_warriors_core::map::MoveResult;
use spore_warriors_core::wrappings::{Enemy, Point};
use spore_warriors_resources::parse_to_binary;
use wasm_bindgen::prelude::*;

lazy_static! {
    static ref GAME: Mutex<Option<Game>> = Mutex::new(None);
    static ref WARRIOR_CONTEXT: Mutex<Option<WarriorContext>> = Mutex::new(None);
    static ref WARRIOR_DECK_CONTEXT: Mutex<Option<WarriorDeckContext>> = Mutex::new(None);
    static ref PVE_BATTLE: Mutex<Option<MapBattlePVE>> = Mutex::new(None);
}

macro_rules! unwrap_result {
    ($val:expr) => {
        match $val {
            Ok(v) => v,
            Err(e) => return Err(JsValue::from_str(&e.to_string()).into()),
        }
    };
}

macro_rules! unwrap_option {
    ($val:ident . $meth:ident ()) => {
        match $val.$meth() {
            Some(v) => v,
            None => {
                return Err(JsValue::from_str(&format!("none {} option", stringify!($val))).into())
            }
        }
    };
}

macro_rules! error {
    ($err:expr) => {
        JsValue::from_str($err).into()
    };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmGame {}

#[wasm_bindgen]
impl WasmGame {
    pub fn get_potion(&self) -> Result<JsValue, Error> {
        let game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_ref());
        if let Some(potion) = &game.potion {
            serde_wasm_bindgen::to_value(potion)
        } else {
            Ok(JsValue::NULL)
        }
    }

    pub fn get_map(&self) -> WasmMap {
        WasmMap::default()
    }

    pub fn create_session(
        &self,
        player_id: u16,
        point_x: u8,
        point_y: u8,
        raw_potion: &[u8],
    ) -> Result<(), Error> {
        let mut warrior_context = unwrap_result!(WARRIOR_CONTEXT.lock());
        let mut warrior_deck_context = unwrap_result!(WARRIOR_DECK_CONTEXT.lock());
        if warrior_context.is_some() || warrior_deck_context.is_some() {
            return Err(error!("warrior or deck have already been initialized"));
        }
        let mut game = unwrap_result!({ GAME.lock() });
        let game = unwrap_option!(game.as_mut());
        let raw_potion = if raw_potion.is_empty() {
            None
        } else {
            Some(raw_potion.to_vec())
        };
        let point = Point {
            x: point_x,
            y: point_y,
        };
        let (warrior, deck) = unwrap_result!(game.new_session(player_id, point, raw_potion));
        *warrior_context = Some(warrior);
        *warrior_deck_context = Some(deck);
        Ok(())
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmMap {}

#[wasm_bindgen]
impl WasmMap {
    pub fn get_profile(&self) -> Result<JsValue, Error> {
        let game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_ref());
        serde_wasm_bindgen::to_value(&game.map)
    }

    pub fn get_warrior_profile(&self) -> Result<JsValue, Error> {
        let warrior = unwrap_result!(WARRIOR_CONTEXT.lock());
        let warrior = unwrap_option!(warrior.as_ref());
        serde_wasm_bindgen::to_value(&warrior)
    }

    pub fn get_warrior_deck_profile(&self) -> Result<JsValue, Error> {
        let deck = unwrap_result!(WARRIOR_DECK_CONTEXT.lock());
        let deck = unwrap_option!(deck.as_ref());
        serde_wasm_bindgen::to_value(&deck)
    }

    pub fn peak_movement(&self, point_x: u8, point_y: u8) -> Result<JsValue, Error> {
        let mut game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_mut());
        let mut warrior = unwrap_result!(WARRIOR_CONTEXT.lock());
        let warrior = unwrap_option!(warrior.as_mut());

        let point = (point_x, point_y).into();
        let node = unwrap_result!(game.map.peak_upcoming_movment(warrior, point));
        if let Some(node) = node {
            serde_wasm_bindgen::to_value(node)
        } else {
            Ok(JsValue::NULL)
        }
    }

    pub fn move_player(
        &self,
        point_x: u8,
        point_y: u8,
        selections: Vec<u8>,
    ) -> Result<JsValue, Error> {
        let mut game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_mut());
        let mut warrior = unwrap_result!(WARRIOR_CONTEXT.lock());
        let mut warrior = unwrap_option!(warrior.as_mut());
        let mut deck = unwrap_result!(WARRIOR_DECK_CONTEXT.lock());
        let mut deck = unwrap_option!(deck.as_mut());
        let point = (point_x, point_y).into();

        let user_imported = selections.into_iter().map(|v| v as usize).collect();
        let move_result = unwrap_result!(game.map.move_to(
            &mut warrior,
            &mut deck,
            point,
            user_imported,
            &mut game.controller,
        ));
        let js_value = serde_wasm_bindgen::to_value(&move_result);
        if let MoveResult::Fight(battle) = move_result {
            let mut global_battle = unwrap_result!(PVE_BATTLE.lock());
            if global_battle.is_some() {
                return Err(error!("battle already triggered from map"));
            }
            *global_battle = Some(battle);
        }
        js_value
    }

    pub fn create_pve_battle(&self) -> Result<WasmBattle, Error> {
        if unwrap_result!(PVE_BATTLE.lock()).is_none() {
            return Err(error!("no battle triggered from map"));
        }
        Ok(WasmBattle::default())
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmBattle {}

#[wasm_bindgen]
impl WasmBattle {
    pub fn start(&self) -> Result<Vec<JsValue>, Error> {
        let mut game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_mut());
        let mut battle = unwrap_result!(PVE_BATTLE.lock());
        let battle = unwrap_option!(battle.as_mut());
        let (output, logs) = battle
            .start(&mut game.controller)
            .map_err::<Error, _>(|e| error!(&e.to_string()))?;
        [
            serde_wasm_bindgen::to_value(&output),
            serde_wasm_bindgen::to_value(&logs),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
    }

    pub fn iterate(&self, input: JsValue) -> Result<Vec<JsValue>, Error> {
        let mut game = unwrap_result!(GAME.lock());
        let game = unwrap_option!(game.as_mut());
        let mut battle = unwrap_result!(PVE_BATTLE.lock());
        let battle = unwrap_option!(battle.as_mut());
        let operations: Vec<IterationInput> = serde_wasm_bindgen::from_value(input)?;
        let (output, logs) = battle
            .run(operations, &mut game.controller)
            .map_err::<Error, _>(|e| error!(&e.to_string()))?;
        [
            serde_wasm_bindgen::to_value(&output),
            serde_wasm_bindgen::to_value(&logs),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
    }

    pub fn check_peak_target(&self, selection: JsValue) -> Result<bool, Error> {
        let mut battle = unwrap_result!(PVE_BATTLE.lock());
        let battle = unwrap_option!(battle.as_mut());
        let selection: Selection = serde_wasm_bindgen::from_value(selection)?;
        battle
            .peak_target(selection)
            .map_err(|e| error!(&e.to_string()))
    }

    pub fn destroy(self) -> Result<(), Error> {
        let mut battle = unwrap_result!(PVE_BATTLE.lock());
        let battle = unwrap_option!(battle.take());
        let (warrior, deck, _) = battle
            .destroy()
            .map_err::<Error, _>(|e| error!(&e.to_string()))?;
        let mut global_warrior = unwrap_result!(WARRIOR_CONTEXT.lock());
        let mut global_deck = unwrap_result!(WARRIOR_DECK_CONTEXT.lock());
        *global_warrior = Some(warrior);
        *global_deck = Some(deck);
        Ok(())
    }
}

#[wasm_bindgen]
pub fn create_game(raw_resource_pool: &[u8], seed: u64) -> Result<WasmGame, Error> {
    let mut global_game = unwrap_result!(GAME.lock());
    if global_game.is_some() {
        return Err(error!("game instance has already been initailized"));
    }
    let game = unwrap_result!(Game::new(&raw_resource_pool.to_vec(), seed));
    *global_game = Some(game);
    Ok(WasmGame::default())
}

#[wasm_bindgen]
pub fn create_standalone_battle(
    warrior: JsValue,
    warrior_deck: JsValue,
    enemies: JsValue,
) -> Result<WasmBattle, Error> {
    let mut global_battle = unwrap_result!(PVE_BATTLE.lock());
    let player: WarriorContext = serde_wasm_bindgen::from_value(warrior)?;
    let player_deck: WarriorDeckContext = serde_wasm_bindgen::from_value(warrior_deck)?;
    let enemies: Vec<Enemy> = serde_wasm_bindgen::from_value(enemies)?;
    let battle = MapBattlePVE::create(player, player_deck, enemies)
        .map_err::<Error, _>(|e| error!(&e.to_string()))?;
    *global_battle = Some(battle);
    Ok(WasmBattle::default())
}

#[wasm_bindgen]
pub fn generate_resource_binary(
    action_pool: String,
    card_pool: String,
    system_pool: String,
    enemy_pool: String,
    loot_pool: String,
    scene_pool: String,
    warrior_pool: String,
) -> Result<Vec<u8>, Error> {
    parse_to_binary(
        &action_pool,
        &card_pool,
        &system_pool,
        &enemy_pool,
        &loot_pool,
        &scene_pool,
        &warrior_pool,
    )
    .map_err(|e| error!(&e.to_string()))
}
