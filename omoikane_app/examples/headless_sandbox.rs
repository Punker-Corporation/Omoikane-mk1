use keisan::Vector2;
use omoikane_app::{App, AppOptions, CpuFrameOptions, RenderFrameOptions, SandboxEntityOptions};
use shinobi::ClientRunLevel;

fn main() -> Result<(), String> {
    let mut app = App::new(AppOptions {
        user_id: "sandbox".to_string(),
        username: "sandbox".to_string(),
        ..AppOptions::default()
    });

    app.startup();
    let sandbox = app.spawn_sandbox_entity(SandboxEntityOptions {
        position: Vector2::new(2.0, -1.0),
        appearance_name: "sandbox_sprite".to_string(),
        ..SandboxEntityOptions::default()
    });
    app.run_ticks(5);

    let state = app.app_state();
    let frame = app
        .build_registered_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
        .map_err(|error| format!("{error:?}"))?;

    println!(
        "headless_sandbox started={} ticks_run={} current_tick={} server={:?} client={:?} entity={} position=({}, {}) appearance={} sprites={} queued_draws={} command_lists={}",
        state.started,
        state.ticks_run,
        state.current_tick.value,
        state.server_state,
        state.client_run_level,
        sandbox.entity,
        sandbox.position.x,
        sandbox.position.y,
        sandbox.appearance_name,
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

    if !app.client().entity_exists(sandbox.entity) {
        return Err(format!(
            "client did not receive sandbox entity {}",
            sandbox.entity
        ));
    }

    app.shutdown();
    Ok(())
}
