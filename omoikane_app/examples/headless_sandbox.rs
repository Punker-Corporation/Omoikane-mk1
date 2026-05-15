use daikoku::BoundKeyState;
use keisan::{Box2, Vector2};
use omoikane_app::{
    App, AppOptions, CpuFrameOptions, OmoikaneProjectConfig, ProjectColorConfig,
    ProjectResourceConfig, ProjectSceneConfig, ProjectSceneEntityConfig,
    ProjectSceneInputBindingConfig,
};
use shinobi::ClientRunLevel;

fn main() -> Result<(), String> {
    let mut app = App::new(AppOptions {
        user_id: "sandbox".to_string(),
        username: "sandbox".to_string(),
        ..AppOptions::default()
    });

    app.startup();
    let project = OmoikaneProjectConfig {
        name: "Headless Sandbox Project".to_string(),
        resources: ProjectResourceConfig::default(),
        scenes: vec![ProjectSceneConfig {
            name: "main".to_string(),
            camera: 1,
            sandbox_texture: 1,
            world_view: Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
            viewport_size: Vector2::new(1280.0, 720.0),
            controlled_entity: Some("player".to_string()),
            input_bindings: vec![ProjectSceneInputBindingConfig {
                action: "move_right".to_string(),
                function: "MoveRight".to_string(),
            }],
            sprite_size: Vector2::ONE,
            sprite_tint: ProjectColorConfig::default(),
            sprite_depth: 0.0,
            sprites: Vec::new(),
            dynamic_entities: vec![ProjectSceneEntityConfig {
                id: "player".to_string(),
                appearance_name: "sandbox_sprite".to_string(),
                texture: 1,
                position: Vector2::new(2.0, -1.0),
                rotation: 0.0,
                prototype: Some("sandbox_sprite".to_string()),
                attach_local_player: false,
                physics: None,
                size: Vector2::ONE,
                tint: ProjectColorConfig::default(),
                depth: 0.0,
            }],
        }],
    };

    let spawned = app
        .spawn_project_scene_entities(&project, "main")
        .map_err(|error| format!("{error:?}"))?;
    let player = spawned
        .first()
        .ok_or_else(|| "project scene did not spawn a player entity".to_string())?
        .entity;
    app.run_ticks(2);
    app.handle_project_scene_input(&project, "main", "move_right", BoundKeyState::Down)
        .map_err(|error| format!("{error:?}"))?;
    app.run_ticks(3);

    let state = app.app_state();
    let resource_config = project
        .to_cpu_frame_resource_config(CpuFrameOptions::default())
        .map_err(|error| format!("{error:?}"))?;
    let frame = app
        .build_project_scene_registered_cpu_frame(&project, "main", resource_config)
        .map_err(|error| format!("{error:?}"))?;
    let player_position = frame
        .extract()
        .sprites()
        .first()
        .map(|sprite| sprite.position())
        .ok_or_else(|| "project scene did not emit a player sprite".to_string())?;

    println!(
        "headless_sandbox started={} ticks_run={} current_tick={} server={:?} client={:?} entity={} position=({}, {}) sprites={} queued_draws={} command_lists={}",
        state.started,
        state.ticks_run,
        state.current_tick.value,
        state.server_state,
        state.client_run_level,
        player,
        player_position.x,
        player_position.y,
        frame.extract().sprites().len(),
        frame.queued().draws().len(),
        frame.submission().command_lists().len()
    );

    if state.client_run_level != ClientRunLevel::InGame {
        return Err(format!(
            "client did not apply server state: {:?}",
            state.client_run_level
        ));
    }

    if !app.client().entity_exists(player) {
        return Err(format!("client did not receive sandbox entity {player}"));
    }

    if player_position.x <= 2.0 {
        return Err(format!(
            "project scene input did not move player right: x={}",
            player_position.x
        ));
    }

    app.shutdown();
    Ok(())
}
