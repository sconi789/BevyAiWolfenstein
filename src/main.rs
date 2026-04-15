use bevy::prelude::*;
use bevy::pbr::{FogFalloff, FogSettings};
use bevy_rapier3d::prelude::*;
use bevy::ecs::system::SystemParam;
use harness::{LevelPlan, Piece};
use hermes::{AgentRole, HermesEvent, HermesPlugin, HermesTopic};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    MainMenu,
    InGame,
    About,
}

const ABOUT_ZH: &str = include_str!("../docs/ABOUT.zh.md");
const ABOUT_JA: &str = include_str!("../docs/ABOUT.ja.md");
const ABOUT_EN: &str = include_str!("../docs/ABOUT.en.md");

#[derive(Resource, Default)]
struct Inventory {
    has_key: bool,
}

#[derive(Resource)]
struct PlayerLoadout {
    weapon: WeaponType,
    shotgun_unlocked: bool,
    shotgun_ammo: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeaponType {
    Pistol,
    Shotgun,
}

#[derive(Resource)]
struct LevelPlanRes(LevelPlan);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerBody;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Health {
    hp: i32,
}

#[derive(Component)]
struct EnemyAi {
    speed: f32,
    attack_range: f32,
    attack_cooldown_s: f32,
    damage: i32,
    cooldown_left_s: f32,
}

#[derive(Component)]
struct Boss;

#[derive(Component)]
struct Minion;

#[derive(Component)]
struct Projectile {
    vel: Vec3,
    damage: i32,
    left_s: f32,
}

#[derive(Component)]
struct BossAi {
    // 常规慢火球
    fireball_cd_s: f32,
    // 追踪弹齐射：一次 5 发，3 秒打完
    volley_cd_s: f32,
    volley_left: i32,
    volley_interval_s: f32,
    volley_tick_s: f32,
    // 召唤
    summon_cooldown_s: f32,
    summon_since_s: f32,
}

#[derive(Component)]
struct Weapon;

#[derive(Component)]
struct GunMuzzle;

#[derive(Component)]
struct EnemyAnim {
    walk_seq: Vec<u32>,
    hit_seq: Vec<u32>,
    die_seq: Vec<u32>,
    state: EnemyAnimState,
    frame_i: usize,
    // Hit 状态剩余时间（播完一轮也会回 walk；用这个避免超高 fps 时一闪而过）
    hit_left_s: f32,
    // Death：播完最后一帧后停留计时
    death_hold_left_s: f32,
    fps: f32,
    acc: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnemyAnimState {
    Walk,
    Hit,
    Die,
}

// enemy_cc0.png 的真实帧矩形（像素坐标），按 y 再按 x 排序
// size: 3888x2656
const ENEMY_FRAME_RECTS_PX: [(u32, u32, u32, u32); 12] = [
    (172, 84, 465, 669),
    (728, 84, 453, 641),
    (1272, 92, 469, 649),
    (1836, 116, 481, 637),
    (164, 978, 489, 665),
    (788, 978, 489, 681),
    (1396, 1010, 493, 669),
    (84, 1806, 557, 725),
    (820, 1834, 569, 661),
    (1632, 1966, 601, 545),
    (2396, 2062, 633, 473),
    (3092, 2310, 673, 245),
];

fn enemy_frame_uv(frame_idx: u32) -> (Vec2, Vec2) {
    let (x, y, w, h) = ENEMY_FRAME_RECTS_PX[frame_idx as usize];
    let tw = 3888.0;
    let th = 2656.0;
    let u0 = x as f32 / tw;
    let u1 = (x + w) as f32 / tw;
    let v0 = y as f32 / th;
    let v1 = (y + h) as f32 / th;
    (Vec2::new(u0, v0), Vec2::new(u1, v1))
}

// 你可以手动改这三组序列来调整动画“顺序/选帧”
// 帧编号对应 ENEMY_FRAME_RECTS_PX 的 0..11（不是网格帧号）
const ENEMY_WALK_SEQ: [u32; 4] = [0, 1, 2, 3];
const ENEMY_HIT_SEQ: [u32; 3] = [4, 5, 6];
// 死亡最后一帧（11）是“横向大面积血泊”，在立牌 billboard 上会显得像“一大坨”；
// 这里改为停在 10（跪倒/趴倒前的更清晰帧），观感更稳定。
const ENEMY_DIE_SEQ: [u32; 4] = [7, 8, 9, 10];

#[derive(Resource)]
struct EnemySpriteConfig {
    v_flip: bool,
}

#[derive(Component)]
struct HitFlash {
    left_s: f32,
}

#[derive(Component)]
struct TimedDespawn {
    left_s: f32,
}

#[derive(Component)]
struct FloatingPickupText;

#[derive(Component)]
struct ItemPickup {
    kind: ItemKind,
}

#[derive(Clone, Copy)]
enum ItemKind {
    Key,
    Health(i32),
    Shotgun { ammo: i32 },
    AmmoShells(i32),
}

#[derive(Component)]
struct Door {
    locked: bool,
}

#[derive(Component)]
struct Key;

#[derive(Component)]
struct FpsLook {
    yaw: f32,
    pitch: f32,
    sensitivity: f32,
}

#[derive(Component)]
struct MainMenuUi;

#[derive(Component)]
enum MenuButton {
    Start,
    About,
    Quit,
}

#[derive(Component)]
struct AboutUiRoot;

#[derive(Component)]
struct AboutUiCamera;

#[derive(Component)]
struct AboutText;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
enum AboutLang {
    #[default]
    Zh,
    Ja,
    En,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum AboutButton {
    Back,
    Lang(AboutLang),
}

#[derive(Resource)]
struct PlayerStats {
    hp: i32,
}

#[derive(Resource, Default)]
struct PlayerConsumables {
    medkits: i32,
}

#[derive(Resource)]
struct FloorState {
    floor: u32,
}

#[derive(Component)]
struct HudUi;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct CrosshairUi;

#[derive(Component)]
struct WorldEntity;

#[derive(Component)]
struct Exit;

#[derive(Resource, Clone)]
struct MazeData {
    tiles_w: u32,
    tiles_h: u32,
    // true = wall
    tiles: Vec<bool>,
    tile_size_world: f32,
    origin_world: Vec3,
}

#[derive(Resource)]
struct MinimapState {
    visible: bool,
}

#[derive(Component)]
struct MinimapUi;

#[derive(Component)]
struct MinimapTile {
    idx: usize,
}

#[derive(Component)]
struct MinimapPlayer;

#[derive(Component)]
struct MinimapEnemy;

#[derive(Resource, Default)]
struct MinimapEnemyPool {
    markers: Vec<Entity>,
}

#[derive(Resource, Clone, Copy)]
struct PlayerSpawn {
    world_pos: Vec3,
}

#[derive(Resource, Clone, Copy)]
struct ExitPos {
    world_pos: Vec3,
}

#[derive(Clone, Copy)]
enum DoorAxis {
    Ns,
    Ew,
}

#[derive(Resource, Clone)]
struct AudioAssets {
    music: Handle<AudioSource>,
    shoot: Handle<AudioSource>,
    pickup: Handle<AudioSource>,
    hit_variants: [Handle<AudioSource>; 5],
    impact: Handle<AudioSource>,
    door_open: Handle<AudioSource>,
    teleport: Handle<AudioSource>,
    footstep: Handle<AudioSource>,
    monster: Handle<AudioSource>,
}

#[derive(Component)]
struct MusicTag;

#[derive(Resource)]
struct EnemyPreview {
    enabled: bool,
    frame: u32,
}

#[derive(Component)]
struct EnemyPreviewTag;

#[derive(Resource)]
struct FootstepTimer {
    t: Timer,
}

#[derive(Resource, Default)]
struct FireState {
    cooldown_s: f32,
}

#[derive(Resource, Default)]
struct PickupToast {
    text: String,
    left_s: f32,
}

#[derive(Component)]
struct PickupToastText;

fn main() {
    let plan = harness::read_level_plan_from_path("assets/level_plan.yaml")
        .expect("Harness gate failed for assets/level_plan.yaml");

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.025)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.12, 0.12, 0.14),
            brightness: 260.0,
        })
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "assets".to_string(),
            ..default()
        }))
        .init_state::<AppState>()
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(HermesPlugin)
        .insert_resource(Inventory::default())
        .insert_resource(PlayerStats { hp: 100 })
        .insert_resource(PlayerConsumables::default())
        .insert_resource(PlayerLoadout {
            weapon: WeaponType::Pistol,
            shotgun_unlocked: false,
            shotgun_ammo: 0,
        })
        .insert_resource(FloorState { floor: 1 })
        .insert_resource(MinimapState { visible: true })
        .insert_resource(MinimapEnemyPool::default())
        .insert_resource(PlayerSpawn {
            world_pos: Vec3::new(0.0, 1.0, 6.0),
        })
        .insert_resource(ExitPos {
            world_pos: Vec3::new(0.0, 0.6, 0.0),
        })
        .insert_resource(EnemyPreview {
            enabled: false,
            frame: 0,
        })
        .insert_resource(EnemySpriteConfig { v_flip: true })
        .insert_resource(FootstepTimer {
            // 高速移动下也能听到连续脚步
            t: Timer::from_seconds(0.22, TimerMode::Repeating),
        })
        .insert_resource(FireState::default())
        .insert_resource(PickupToast::default())
        .insert_resource(AboutLang::default())
        .add_systems(Startup, load_audio_assets)
        .insert_resource(LevelPlanRes(plan))
        .add_systems(OnEnter(AppState::MainMenu), setup_menu)
        .add_systems(Update, menu_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), cleanup_menu)
        .add_systems(OnEnter(AppState::About), setup_about)
        .add_systems(Update, about_interaction.run_if(in_state(AppState::About)))
        .add_systems(OnExit(AppState::About), cleanup_about)
        .add_systems(OnEnter(AppState::InGame), setup_game)
        .add_systems(OnExit(AppState::InGame), cleanup_game)
        .add_systems(Update, fps_look.run_if(in_state(AppState::InGame)))
        .add_systems(Update, fps_move.run_if(in_state(AppState::InGame)))
        .add_systems(Update, weapon_switch.run_if(in_state(AppState::InGame)))
        .add_systems(Update, shoot.run_if(in_state(AppState::InGame)))
        .add_systems(Update, enemy_ai.run_if(in_state(AppState::InGame)))
        .add_systems(Update, tick_enemy_anim.run_if(in_state(AppState::InGame)))
        .add_systems(Update, boss_ai.run_if(in_state(AppState::InGame)))
        .add_systems(Update, projectiles.run_if(in_state(AppState::InGame)))
        .add_systems(Update, boss_victory_check.run_if(in_state(AppState::InGame)))
        // 先更新 UI，再处理换层，避免 update_minimap 对“即将 despawn 的 UI 根节点”写入子节点导致崩溃
        .add_systems(Update, update_minimap.run_if(in_state(AppState::InGame)))
        .add_systems(Update, check_exit_and_advance_floor.run_if(in_state(AppState::InGame)))
        .add_systems(Update, tick_timed_despawn.run_if(in_state(AppState::InGame)))
        .add_systems(Update, update_hud.run_if(in_state(AppState::InGame)))
        .add_systems(Update, pickup_items.run_if(in_state(AppState::InGame)))
        .add_systems(Update, pickup_key.run_if(in_state(AppState::InGame)))
        .add_systems(Update, try_open_door.run_if(in_state(AppState::InGame)))
        .add_systems(Update, hermes_debug_log.run_if(in_state(AppState::InGame)))
        .add_systems(Update, enemy_preview_controls.run_if(in_state(AppState::InGame)))
        .add_systems(Update, enemy_preview_update.run_if(in_state(AppState::InGame)))
        .add_systems(Update, footsteps.run_if(in_state(AppState::InGame)))
        .add_systems(Update, face_floating_pickup_text.run_if(in_state(AppState::InGame)))
        .add_systems(Update, update_pickup_toast.run_if(in_state(AppState::InGame)))
        .add_systems(Update, use_medkit.run_if(in_state(AppState::InGame)))
        .run();
}

fn footsteps(
    time: Res<Time>,
    mut ft: ResMut<FootstepTimer>,
    audio: Res<AudioAssets>,
    player: Query<&Velocity, With<PlayerBody>>,
    mut commands: Commands,
) {
    let Ok(v) = player.get_single() else { return };
    let speed = Vec2::new(v.linvel.x, v.linvel.z).length();
    if speed < 0.8 {
        ft.t.reset();
        return;
    }
    ft.t.tick(time.delta());
    if ft.t.just_finished() {
        commands.spawn(AudioBundle {
            source: audio.footstep.clone(),
            settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.85)),
        });
    }
}

fn enemy_preview_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut p: ResMut<EnemyPreview>,
    mut cfg: ResMut<EnemySpriteConfig>,
) {
    if keys.just_pressed(KeyCode::F1) {
        p.enabled = !p.enabled;
    }
    if keys.just_pressed(KeyCode::F2) {
        cfg.v_flip = !cfg.v_flip;
    }
    if !p.enabled {
        return;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        p.frame = p.frame.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        p.frame = (p.frame + 1).min(11);
    }
}

fn enemy_preview_update(
    mut commands: Commands,
    preview: Res<EnemyPreview>,
    cfg: Res<EnemySpriteConfig>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<(Entity, &Handle<Mesh>), With<EnemyPreviewTag>>,
) {
    if !preview.is_changed() {
        return;
    }
    if !preview.enabled {
        for (e, _) in &q {
            commands.entity(e).despawn_recursive();
        }
        return;
    }
    let tex: Handle<Image> = assets.load("textures/enemy_cc0.png");
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(tex),
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let mut uv = enemy_frame_uv(preview.frame);
    if cfg.v_flip {
        uv = (Vec2::new(uv.0.x, uv.1.y), Vec2::new(uv.1.x, uv.0.y));
    }
    if let Some((e, mesh_handle)) = q.iter().next() {
        if let Some(m) = meshes.get_mut(mesh_handle) {
            m.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[uv.0.x, uv.0.y], [uv.1.x, uv.0.y], [uv.1.x, uv.1.y], [uv.0.x, uv.1.y]],
            );
        }
        commands.entity(e).insert(Visibility::Visible);
    } else {
        let mesh = meshes.add(make_billboard_mesh(2.2, 3.0, uv));
        commands.spawn((
            PbrBundle {
                mesh,
                material: mat,
                transform: Transform::from_xyz(0.0, 2.4, -3.2),
                ..default()
            },
            EnemyPreviewTag,
        ));
    }
}

fn load_audio_assets(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(AudioAssets {
        music: assets.load("audio/music_cc0.wav"),
        // 枪声用短音频，避免 long wav 被连射“听成一直在播”
        // 如果你后续补回真正的 shoot_cc0.wav，可以改回 assets.load("audio/shoot_cc0.wav")
        shoot: assets.load("audio/hit_pack_cc0/ogg/hit2.ogg"),
        // 拾取音：用更短的 hit pack 音效代替长音频
        pickup: assets.load("audio/hit_pack_cc0/ogg/hit5.ogg"),
        // 命中怪物：hit1~hit5 随机；命中墙面：hit_cc0
        hit_variants: [
            assets.load("audio/hit_pack_cc0/ogg/hit1.ogg"),
            assets.load("audio/hit_pack_cc0/ogg/hit2.ogg"),
            assets.load("audio/hit_pack_cc0/ogg/hit3.ogg"),
            assets.load("audio/hit_pack_cc0/ogg/hit4.ogg"),
            assets.load("audio/hit_pack_cc0/ogg/hit5.ogg"),
        ],
        impact: assets.load("audio/hit_cc0.wav"),
        door_open: assets.load("audio/door_open_cc0.wav"),
        teleport: assets.load("audio/teleport_cc0.wav"),
        footstep: assets.load("audio/footstep_pack_cc0/Ejimas1.ogg"),
        monster: assets.load("audio/monster_pack_cc0/monster.6.ogg"),
    });
}

fn setup_menu(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((Camera2dBundle::default(), MainMenuUi));
    let ui_font: Handle<Font> = assets.load("fonts/NotoSansCJKsc-Regular.otf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                ..default()
            },
            MainMenuUi,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                "Monster Hell 怪物地狱",
                TextStyle {
                    font: ui_font.clone(),
                    font_size: 48.0,
                    color: Color::srgb(0.95, 0.95, 0.98),
                    ..default()
                },
            ));

            p.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(260.0),
                        height: Val::Px(56.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                    ..default()
                },
                MenuButton::Start,
            ))
            .with_children(|b| {
                b.spawn(TextBundle::from_section(
                    "开始游戏",
                    TextStyle {
                        font: ui_font.clone(),
                        font_size: 28.0,
                        color: Color::srgb(0.95, 0.95, 0.98),
                        ..default()
                    },
                ));
            });

            p.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(260.0),
                        height: Val::Px(56.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                    ..default()
                },
                MenuButton::About,
            ))
            .with_children(|b| {
                b.spawn(TextBundle::from_section(
                    "说明 / About",
                    TextStyle {
                        font: ui_font.clone(),
                        font_size: 24.0,
                        color: Color::srgb(0.95, 0.95, 0.98),
                        ..default()
                    },
                ));
            });

            p.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(260.0),
                        height: Val::Px(56.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                    ..default()
                },
                MenuButton::Quit,
            ))
            .with_children(|b| {
                b.spawn(TextBundle::from_section(
                    "退出",
                    TextStyle {
                        font: ui_font,
                        font_size: 28.0,
                        color: Color::srgb(0.95, 0.95, 0.98),
                        ..default()
                    },
                ));
            });
        });
}

fn cleanup_menu(mut commands: Commands, q: Query<Entity, With<MainMenuUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn menu_interaction(
    mut next: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
    mut buttons: Query<(&Interaction, &MenuButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, kind, mut bg) in &mut buttons {
        match *interaction {
            Interaction::None => {
                *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.22));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgb(0.26, 0.26, 0.32));
            }
            Interaction::Pressed => {
                match kind {
                    MenuButton::Start => next.set(AppState::InGame),
                    MenuButton::About => next.set(AppState::About),
                    MenuButton::Quit => {
                        app_exit.send(AppExit::Success);
                    }
                }
            }
        }
    }
}

fn setup_about(mut commands: Commands, assets: Res<AssetServer>, lang: Res<AboutLang>) {
    commands.spawn((Camera2dBundle::default(), AboutUiCamera));
    let ui_font: Handle<Font> = assets.load("fonts/NotoSansCJKsc-Regular.otf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.88)),
                ..default()
            },
            AboutUiRoot,
        ))
        .with_children(|root| {
            // Top bar
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|top| {
                // Back
                top.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(160.0),
                            height: Val::Px(44.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                        ..default()
                    },
                    AboutButton::Back,
                ))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section(
                        "返回 / Back",
                        TextStyle {
                            font: ui_font.clone(),
                            font_size: 22.0,
                            color: Color::srgb(0.95, 0.95, 0.98),
                            ..default()
                        },
                    ));
                });

                // Language links
                top.spawn(NodeBundle {
                    style: Style {
                        column_gap: Val::Px(10.0),
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|langs| {
                    let mk_lang_btn = |p: &mut ChildBuilder, label: &str, l: AboutLang| {
                        p.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(120.0),
                                    height: Val::Px(44.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                                ..default()
                            },
                            AboutButton::Lang(l),
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section(
                                label,
                                TextStyle {
                                    font: ui_font.clone(),
                                    font_size: 20.0,
                                    color: Color::srgb(0.95, 0.95, 0.98),
                                    ..default()
                                },
                            ));
                        });
                    };

                    mk_lang_btn(langs, "中文", AboutLang::Zh);
                    mk_lang_btn(langs, "日本語", AboutLang::Ja);
                    mk_lang_btn(langs, "English", AboutLang::En);
                });
            });

            // Content area (clipped; keeps layout stable across resolutions)
            root.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        padding: UiRect::all(Val::Px(12.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.10, 0.70)),
                    ..default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    TextBundle::from_section(
                        about_text_for(*lang),
                        TextStyle {
                            font: ui_font,
                            font_size: 18.0,
                            color: Color::srgb(0.94, 0.94, 0.97),
                            ..default()
                        },
                    )
                    .with_text_justify(JustifyText::Left),
                    AboutText,
                ));
            });
        });
}

fn cleanup_about(
    mut commands: Commands,
    root_q: Query<Entity, With<AboutUiRoot>>,
    cam_q: Query<Entity, With<AboutUiCamera>>,
) {
    if let Ok(e) = root_q.get_single() {
        commands.entity(e).despawn_recursive();
    }
    if let Ok(e) = cam_q.get_single() {
        commands.entity(e).despawn_recursive();
    }
}

fn about_interaction(
    mut next: ResMut<NextState<AppState>>,
    mut lang: ResMut<AboutLang>,
    mut buttons: Query<(&Interaction, &AboutButton, &mut BackgroundColor), Changed<Interaction>>,
    mut text_q: Query<&mut Text, With<AboutText>>,
) {
    for (interaction, kind, mut bg) in &mut buttons {
        match *interaction {
            Interaction::None => {
                *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.22));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgb(0.26, 0.26, 0.32));
            }
            Interaction::Pressed => match *kind {
                AboutButton::Back => next.set(AppState::MainMenu),
                AboutButton::Lang(l) => {
                    if *lang != l {
                        *lang = l;
                        if let Ok(mut t) = text_q.get_single_mut() {
                            t.sections[0].value = about_text_for(l).to_string();
                        }
                    }
                }
            },
        }
    }
}

fn about_text_for(lang: AboutLang) -> &'static str {
    match lang {
        AboutLang::Zh => ABOUT_ZH,
        AboutLang::Ja => ABOUT_JA,
        AboutLang::En => ABOUT_EN,
    }
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    audio: Res<AudioAssets>,
    plan: Res<LevelPlanRes>,
    floor: Res<FloorState>,
    mut stats: ResMut<PlayerStats>,
    mut loadout: ResMut<PlayerLoadout>,
    mut minimap_state: ResMut<MinimapState>,
    mut spawn: ResMut<PlayerSpawn>,
    mut exit_pos_res: ResMut<ExitPos>,
    music_q: Query<Entity, With<MusicTag>>,
    mut hermes: EventWriter<HermesEvent>,
) {
    let plan = &plan.0;
    let ui_font: Handle<Font> = assets.load("fonts/NotoSansCJKsc-Regular.otf");
    stats.hp = 100;
    commands.insert_resource(PlayerConsumables::default());
    loadout.weapon = WeaponType::Pistol;
    loadout.shotgun_unlocked = false;
    loadout.shotgun_ammo = 0;
    minimap_state.visible = true;

    // Floor 5：Boss 房（固定大空间）。不生成迷宫/出口，击败 Boss 后通关。
    if floor.floor == 5 {
        setup_boss_arena(&mut commands, &mut meshes, &mut materials, &assets, spawn.world_pos);
        hermes.send(HermesEvent {
            topic: HermesTopic::ProducerGate,
            from: AgentRole::Producer,
            message: "进入 Boss 房：Floor 5".to_string(),
        });
        return;
    }

    // 背景音乐：只在首次进入 InGame 时启动，过层不重启
    if music_q.is_empty() {
        commands.spawn((
            AudioBundle {
                source: audio.music.clone(),
                settings: PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::new(0.35),
                    ..default()
                },
            },
            MusicTag,
        ));
    }

    // 先生成迷宫资源，选一个可走格作为出生点，防止卡墙
    let mut rng = StdRng::seed_from_u64(plan.seed ^ (floor.floor as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let maze_gen = generate_rooms_and_corridors_tiles(15, 15, &mut rng);
    let tile_size_world = 2.0;
    let origin_world = Vec3::new(
        -(maze_gen.0 as f32 - 1.0) * tile_size_world * 0.5,
        0.0,
        -(maze_gen.1 as f32 - 1.0) * tile_size_world * 0.5,
    );
    let maze = MazeData {
        tiles_w: maze_gen.0,
        tiles_h: maze_gen.1,
        tiles: maze_gen.2,
        tile_size_world,
        origin_world,
    };
    spawn_maze_walls(&mut commands, &mut meshes, &mut materials, &assets, &maze);
    commands.insert_resource(maze.clone());

    let spawn_pos = pick_spawn_world(&maze, Vec3::new(plan.player_start[0], 0.0, plan.player_start[2]));
    spawn.world_pos = Vec3::new(spawn_pos.x, 1.0, spawn_pos.z);
    let exit_pos = pick_exit_world(&maze, spawn.world_pos, &mut rng);
    exit_pos_res.world_pos = Vec3::new(exit_pos.x, 0.6, exit_pos.z);

    // 随机锁门：放在通往出口的走廊卡点处（需要钥匙才能通过），并在门后放一些道具
    if let (Some(spawn_tile), Some(exit_tile)) = (
        maze_world_to_tile(&maze, Vec3::new(spawn.world_pos.x, 0.0, spawn.world_pos.z)),
        maze_world_to_tile(&maze, Vec3::new(exit_pos_res.world_pos.x, 0.0, exit_pos_res.world_pos.z)),
    ) {
        let dist = maze_bfs_dist(&maze, spawn_tile);
        if let Some((door_tile, axis, door_d)) = pick_locked_door_tile(&maze, &dist, exit_tile) {
            // 生成锁门
            spawn_locked_door(
                &mut commands,
                &mut meshes,
                &mut materials,
                &assets,
                &maze,
                door_tile,
                axis,
                true,
            );
            // 钥匙放在门之前（靠近出生点）
            if let Some(key_tile) = pick_tile_with_dist_range(&maze, &dist, 3, (door_d - 6).max(3), &mut rng) {
                let key_pos = maze_tile_to_world_center(&maze, key_tile.0, key_tile.1, 0.6);
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Sphere::new(0.22)),
                        material: materials.add(StandardMaterial {
                            base_color: Color::srgb(0.95, 0.85, 0.2),
                            emissive: LinearRgba::new(0.9, 0.7, 0.15, 1.0),
                            ..default()
                        }),
                        transform: Transform::from_translation(key_pos),
                        ..default()
                    },
                    ItemPickup { kind: ItemKind::Key },
                    Collider::ball(0.4),
                    Sensor,
                    RigidBody::Fixed,
                    WorldEntity,
                ));
            }
            // 门后放置 2~3 个道具（尽量在更深的位置）
            for _ in 0..rng.gen_range(2..=3) {
                if let Some(treasure_tile) = pick_tile_with_dist_range(&maze, &dist, door_d + 4, door_d + 16, &mut rng) {
                    let p = maze_tile_to_world_center(&maze, treasure_tile.0, treasure_tile.1, 0.3);
                    let roll = rng.gen_range(0..100);
                    if roll < 40 {
                        commands.spawn((
                            PbrBundle {
                                mesh: meshes.add(Cuboid::new(0.35, 0.22, 0.35)),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.2, 0.9, 0.2),
                                    emissive: LinearRgba::new(0.05, 0.25, 0.05, 1.0),
                                    ..default()
                                }),
                                transform: Transform::from_xyz(p.x, 0.3, p.z),
                                ..default()
                            },
                            ItemPickup { kind: ItemKind::Health(25) },
                            Collider::cuboid(0.25, 0.15, 0.25),
                            Sensor,
                            RigidBody::Fixed,
                            WorldEntity,
                        ));
                    } else if roll < 75 {
                        commands.spawn((
                            PbrBundle {
                                mesh: meshes.add(Cuboid::new(0.8, 0.12, 0.28)),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.35, 0.35, 0.38),
                                    emissive: LinearRgba::new(0.02, 0.02, 0.02, 1.0),
                                    ..default()
                                }),
                                transform: Transform::from_xyz(p.x, 0.25, p.z),
                                ..default()
                            },
                            ItemPickup { kind: ItemKind::Shotgun { ammo: 8 } },
                            Collider::cuboid(0.5, 0.15, 0.2),
                            Sensor,
                            RigidBody::Fixed,
                            WorldEntity,
                        ));
                    } else {
                        commands.spawn((
                            PbrBundle {
                                mesh: meshes.add(Cuboid::new(0.5, 0.12, 0.22)),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.55, 0.55, 0.6),
                                    emissive: LinearRgba::new(0.02, 0.02, 0.02, 1.0),
                                    ..default()
                                }),
                                transform: Transform::from_xyz(p.x, 0.25, p.z),
                                ..default()
                            },
                            ItemPickup { kind: ItemKind::AmmoShells(6) },
                            Collider::cuboid(0.35, 0.15, 0.18),
                            Sensor,
                            RigidBody::Fixed,
                            WorldEntity,
                        ));
                    }
                }
            }
        }
    }

    let player_body = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(spawn.world_pos)),
            PlayerBody,
            RigidBody::Dynamic,
            Velocity::zero(),
            Collider::capsule_y(0.55, 0.25),
            LockedAxes::ROTATION_LOCKED,
            GravityScale(1.0),
            Ccd::enabled(),
            Damping {
                linear_damping: 8.0,
                angular_damping: 8.0,
            },
        ))
        .id();

    // 摄像机跟随刚体；视角由摄像机自身旋转决定，移动基于摄像机朝向推力。
    commands.entity(player_body).with_children(|c| {
        c.spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 0.6, 0.0)
                        .looking_at(Vec3::new(0.0, 0.6, -1.0), Vec3::Y),
                    ..default()
                },
                FogSettings {
                    color: Color::srgb(0.02, 0.02, 0.03),
                    directional_light_color: Color::srgb(0.05, 0.05, 0.06),
                    directional_light_exponent: 3.0,
                    falloff: FogFalloff::Linear {
                        start: 3.0,
                        end: 22.0,
                    },
                },
                Player,
                FpsLook {
                    yaw: 0.0,
                    pitch: 0.0,
                    sensitivity: 0.0,
                },
                Weapon,
            ))
            .with_children(|c2| {
                // 简单“枪”模型（挂在相机前方）
                c2.spawn(PbrBundle {
                    mesh: meshes.add(Cuboid::new(0.18, 0.14, 0.35)),
                    material: materials.add(StandardMaterial {
                        base_color: Color::srgb(0.08, 0.08, 0.09),
                        perceptual_roughness: 0.6,
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.23, -0.18, -0.35),
                    ..default()
                });

                // 枪口：子弹(tracer)从这里发射
                c2.spawn((
                    SpatialBundle {
                        transform: Transform::from_xyz(0.23, -0.18, -0.58),
                        ..default()
                    },
                    GunMuzzle,
                ));

                c2.spawn(SpotLightBundle {
                    spot_light: SpotLight {
                        color: Color::srgb(0.95, 0.97, 1.0),
                        intensity: 60_000.0,
                        range: 40.0,
                        outer_angle: 0.55,
                        inner_angle: 0.25,
                        shadows_enabled: true,
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, -0.05, 0.0),
                    ..default()
                });
            });
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::srgb(0.7, 0.75, 0.85),
            illuminance: 3_500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            0.2,
            -1.2,
            0.0,
        )),
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });

    // CC0 贴图（可平铺墙面）。如果贴图加载失败，会回退到 base_color。
    let wall_tex: Handle<Image> = assets.load("textures/wall_cc0.png");
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        base_color: Color::srgb(0.85, 0.85, 0.88),
        // 轻微自发光，避免“看起来像没墙/全黑”的错觉（手电筒照不到也可读）。
        emissive: LinearRgba::new(0.03, 0.03, 0.035, 1.0),
        perceptual_roughness: 0.98,
        ..default()
    });

    let door_tex: Handle<Image> = assets.load("textures/door_cc0.png");
    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(door_tex),
        base_color: Color::srgb(0.82, 0.82, 0.85),
        emissive: LinearRgba::new(0.02, 0.02, 0.025, 1.0),
        perceptual_roughness: 0.95,
        ..default()
    });
    let key_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.8, 0.2),
        emissive: LinearRgba::new(0.6, 0.5, 0.1, 1.0),
        ..default()
    });

    // 基础“房间壳”来自 YAML（用于 Harness gate）
    for piece in &plan.pieces {
        match piece {
            Piece::Floor { pos, size } => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Cuboid::new(size[0], size[1], size[2])),
                        material: floor_mat.clone(),
                        transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                        ..default()
                    },
                    Collider::cuboid(size[0] * 0.5, size[1] * 0.5, size[2] * 0.5),
                    RigidBody::Fixed,
                    WorldEntity,
                ));
            }
            Piece::Wall { pos, size } => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Cuboid::new(size[0], size[1], size[2])),
                        material: wall_mat.clone(),
                        transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                        ..default()
                    },
                    Collider::cuboid(size[0] * 0.5, size[1] * 0.5, size[2] * 0.5),
                    RigidBody::Fixed,
                    WorldEntity,
                ));
            }
            Piece::Door { pos, size, locked } => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Cuboid::new(size[0], size[1], size[2])),
                        material: door_mat.clone(),
                        transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                        ..default()
                    },
                    Door { locked: *locked },
                    Collider::cuboid(size[0] * 0.5, size[1] * 0.5, size[2] * 0.5),
                    RigidBody::Fixed,
                    WorldEntity,
                ));
            }
            Piece::Key { pos } => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Sphere::new(0.18)),
                        material: key_mat.clone(),
                        transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                        ..default()
                    },
                    Key,
                    WorldEntity,
                ));
            }
            Piece::Light {
                pos,
                intensity,
                range,
            } => {
                commands.spawn((
                    PointLightBundle {
                        point_light: PointLight {
                            intensity: *intensity,
                            range: *range,
                            shadows_enabled: true,
                            ..default()
                        },
                        transform: Transform::from_xyz(pos[0], pos[1], pos[2]),
                        ..default()
                    },
                    WorldEntity,
                ));
            }
        }
    }

    // 复用上面创建的 `maze` 与 `rng`

    // 出口（到下一层的触发器）：显眼的发光柱，且在可走格子里
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.2, 3.6, 1.2)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 0.3),
                emissive: LinearRgba::new(0.12, 0.45, 0.16, 1.0),
                ..default()
            }),
            transform: Transform::from_xyz(exit_pos_res.world_pos.x, 1.8, exit_pos_res.world_pos.z),
            ..default()
        },
        Exit,
        Collider::cuboid(0.8, 1.8, 0.8),
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
        RigidBody::Fixed,
        WorldEntity,
    ));
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 22_000.0,
                range: 18.0,
                color: Color::srgb(0.2, 0.9, 0.3),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(exit_pos_res.world_pos.x, 2.6, exit_pos_res.world_pos.z),
            ..default()
        },
        WorldEntity,
    ));

    // HUD
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    top: Val::Px(12.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)),
                ..default()
            },
            HudUi,
        ))
        .with_children(|p| {
            p.spawn((
                TextBundle::from_section(
                    "HP: 100 | Floor: 1",
                    TextStyle {
                        font: ui_font.clone(),
                        font_size: 22.0,
                        color: Color::srgb(0.95, 0.95, 0.98),
                        ..default()
                    },
                ),
                HudText,
            ));
            p.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: ui_font.clone(),
                        font_size: 20.0,
                        color: Color::srgb(1.0, 0.92, 0.75),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                }),
                PickupToastText,
            ));
        });

    // 准星（居中），用于矫正“枪口模型偏斜”带来的瞄准误差
    commands.spawn((
        TextBundle::from_section(
            "+",
            TextStyle {
                font: ui_font.clone(),
                font_size: 28.0,
                color: Color::srgb(0.98, 0.98, 0.98),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            ..default()
        }),
        CrosshairUi,
    ));

    // 敌人（每层递增；序列帧怪物）
    let enemy_count = (floor.floor * 2).min(20);
    let enemy_tex: Handle<Image> = assets.load("textures/enemy_cc0.png");
    let enemy_mat_proto = StandardMaterial {
        base_color_texture: Some(enemy_tex),
        base_color: Color::srgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    };
    let player_safe = spawn.world_pos;
    let mut enemy_placed: Vec<Vec3> = Vec::with_capacity(enemy_count as usize);
    let mut enemy_tries = 0;
    while enemy_placed.len() < enemy_count as usize && enemy_tries < enemy_count as usize * 200 {
        enemy_tries += 1;
        let p = random_walkable_tile_world(&maze, &mut rng);
        if p.distance(Vec3::new(player_safe.x, p.y, player_safe.z)) < 8.0 {
            continue;
        }
        if enemy_placed.iter().any(|q| q.distance(p) < 2.0) {
            continue;
        }
        enemy_placed.push(p);

        let enemy_mat = materials.add(enemy_mat_proto.clone());
        let enemy_mesh = meshes.add(make_billboard_mesh(2.1, 2.9, enemy_frame_uv(0)));
        commands.spawn((
            PbrBundle {
                mesh: enemy_mesh,
                material: enemy_mat,
                transform: Transform::from_xyz(p.x, 0.85, p.z),
                ..default()
            },
            Enemy,
            Health { hp: 30 },
            EnemyAi {
                speed: 1.05,
                attack_range: 1.15,
                attack_cooldown_s: 1.6,
                damage: 3,
                cooldown_left_s: 0.0,
            },
            EnemyAnim {
                // 真实矩形帧序列：walk(0..3), hit(4..6), die(7..11)
                walk_seq: ENEMY_WALK_SEQ.to_vec(),
                hit_seq: ENEMY_HIT_SEQ.to_vec(),
                die_seq: ENEMY_DIE_SEQ.to_vec(),
                state: EnemyAnimState::Walk,
                frame_i: 0,
                hit_left_s: 0.0,
                death_hold_left_s: 0.0,
                fps: 10.0,
                acc: 0.0,
            },
            RigidBody::Dynamic,
            GravityScale(0.0),
            Ccd::enabled(),
            Damping {
                linear_damping: 4.0,
                angular_damping: 8.0,
            },
            Velocity::zero(),
            // 命中判定加宽一点（含手臂区域）
            Collider::capsule_y(0.55, 0.42),
            LockedAxes::ROTATION_LOCKED,
            WorldEntity,
        ));
    }

    // 本层道具/钥匙/锁门由“随机锁门”逻辑统一生成

    hermes.send(HermesEvent {
        topic: HermesTopic::ProducerGate,
        from: AgentRole::Producer,
        message: "Harness gate 已通过：level_plan.yaml".to_string(),
    });
}

fn cleanup_game(
    mut commands: Commands,
    q: Query<Entity, With<WorldEntity>>,
    player: Query<Entity, With<PlayerBody>>,
    music: Query<Entity, With<MusicTag>>,
    crosshair: Query<Entity, With<CrosshairUi>>,
) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
    if let Ok(p) = player.get_single() {
        commands.entity(p).despawn_recursive();
    }
    for e in &music {
        commands.entity(e).despawn_recursive();
    }
    for e in &crosshair {
        commands.entity(e).despawn_recursive();
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct FloorAdvanceParams<'w, 's> {
    rapier: Res<'w, RapierContext>,
    player: Query<'w, 's, (Entity, &'static mut Transform, &'static mut Velocity), With<PlayerBody>>,
    exit: Query<'w, 's, Entity, With<Exit>>,
    floor: ResMut<'w, FloorState>,
    hermes: EventWriter<'w, HermesEvent>,
    commands: Commands<'w, 's>,
    q_world: Query<'w, 's, Entity, With<WorldEntity>>,
    stats: ResMut<'w, PlayerStats>,
    loadout: ResMut<'w, PlayerLoadout>,
    next: ResMut<'w, NextState<AppState>>,
    plan: Res<'w, LevelPlanRes>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    assets: Res<'w, AssetServer>,
    minimap_state: ResMut<'w, MinimapState>,
    minimap_ui: Query<'w, 's, Entity, With<MinimapUi>>,
    spawn: ResMut<'w, PlayerSpawn>,
    exit_pos_res: ResMut<'w, ExitPos>,
    audio: Res<'w, AudioAssets>,
}

fn check_exit_and_advance_floor(mut p: FloorAdvanceParams) {
    let Ok((player_e, mut player_t, mut player_v)) = p.player.get_single_mut() else { return };
    // Floor 5（Boss 房）不依赖 Exit；通关由 boss_victory_check 处理
    if p.floor.floor >= 5 {
        return;
    }
    let Ok(exit_e) = p.exit.get_single() else { return };

    if p.rapier.intersection_pair(player_e, exit_e) != Some(true) {
        return;
    }

    // 进入传送区域音效
    p.commands.spawn(AudioBundle {
        source: p.audio.teleport.clone(),
        settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.8)),
    });

    if p.floor.floor >= 5 {
        p.hermes.send(HermesEvent {
            topic: HermesTopic::ProducerGate,
            from: AgentRole::Producer,
            message: "通关 5 层：返回主菜单".to_string(),
        });
        p.floor.floor = 1;
        p.stats.hp = 100;
        p.next.set(AppState::MainMenu);
        return;
    }

    p.floor.floor += 1;
    p.hermes.send(HermesEvent {
        topic: HermesTopic::ProducerGate,
        from: AgentRole::Producer,
        message: format!("进入下一层：Floor {}", p.floor.floor),
    });

    // 清理本层世界实体（保留玩家）
    for e in &p.q_world {
        p.commands.entity(e).despawn_recursive();
    }
    // 换层后强制重建小地图（否则瓦片还指向上一层）
    for e in &p.minimap_ui {
        p.commands.entity(e).despawn_recursive();
    }
    p.minimap_state.visible = true;
    // 换层不重置武器/弹药/解锁状态（否则无法切枪）

    // 重置玩家位置/速度
    player_t.translation = Vec3::new(0.0, 1.0, 0.0);
    player_v.linvel = Vec3::ZERO;
    player_v.angvel = Vec3::ZERO;

    // Floor 5：直接生成 Boss 大空间，不再生成迷宫/出口/外墙（避免叠加导致 Boss 卡墙/地图异常）
    if p.floor.floor == 5 {
        player_t.translation = Vec3::new(0.0, 1.0, 28.0);
        setup_boss_arena(
            &mut p.commands,
            &mut p.meshes,
            &mut p.materials,
            &p.assets,
            player_t.translation,
        );
        return;
    }

    // 重新生成本层内容：轻量重建（地面+外墙+迷宫+出口+敌人）
    let plan = &p.plan.0;
    let mut rng = StdRng::seed_from_u64(plan.seed ^ (p.floor.floor as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));

    let wall_tex: Handle<Image> = p.assets.load("textures/wall_cc0.png");
    let wall_mat = p.materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        base_color: Color::srgb(0.85, 0.85, 0.88),
        emissive: LinearRgba::new(0.03, 0.03, 0.035, 1.0),
        perceptual_roughness: 0.98,
        ..default()
    });
    let floor_mat = p.materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });

    // 地面（固定）
    let floor_size = Vec3::new(60.0, 0.2, 60.0);
    p.commands.spawn((
        PbrBundle {
            mesh: p.meshes.add(Cuboid::new(floor_size.x, floor_size.y, floor_size.z)),
            material: floor_mat,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Collider::cuboid(floor_size.x * 0.5, floor_size.y * 0.5, floor_size.z * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));

    // 外墙（固定）
    let wall_mesh_x = p.meshes.add(Cuboid::new(0.3, 2.5, 60.0));
    let wall_mesh_z = p.meshes.add(Cuboid::new(60.0, 2.5, 0.3));
    p.commands.spawn((
        PbrBundle {
            mesh: wall_mesh_x.clone(),
            material: wall_mat.clone(),
            transform: Transform::from_xyz(-30.0, 1.0, 0.0),
            ..default()
        },
        Collider::cuboid(0.15, 1.25, 30.0),
        RigidBody::Fixed,
        WorldEntity,
    ));
    p.commands.spawn((
        PbrBundle {
            mesh: wall_mesh_x,
            material: wall_mat.clone(),
            transform: Transform::from_xyz(30.0, 1.0, 0.0),
            ..default()
        },
        Collider::cuboid(0.15, 1.25, 30.0),
        RigidBody::Fixed,
        WorldEntity,
    ));
    p.commands.spawn((
        PbrBundle {
            mesh: wall_mesh_z.clone(),
            material: wall_mat.clone(),
            transform: Transform::from_xyz(0.0, 1.0, -30.0),
            ..default()
        },
        Collider::cuboid(30.0, 1.25, 0.15),
        RigidBody::Fixed,
        WorldEntity,
    ));
    p.commands.spawn((
        PbrBundle {
            mesh: wall_mesh_z,
            material: wall_mat,
            transform: Transform::from_xyz(0.0, 1.0, 30.0),
            ..default()
        },
        Collider::cuboid(30.0, 1.25, 0.15),
        RigidBody::Fixed,
        WorldEntity,
    ));

    let maze_gen = generate_rooms_and_corridors_tiles(15, 15, &mut rng);
    let tile_size_world = 2.0;
    let origin_world = Vec3::new(
        -(maze_gen.0 as f32 - 1.0) * tile_size_world * 0.5,
        0.0,
        -(maze_gen.1 as f32 - 1.0) * tile_size_world * 0.5,
    );
    let maze = MazeData {
        tiles_w: maze_gen.0,
        tiles_h: maze_gen.1,
        tiles: maze_gen.2,
        tile_size_world,
        origin_world,
    };
    spawn_maze_walls(
        &mut p.commands,
        &mut p.meshes,
        &mut p.materials,
        &p.assets,
        &maze,
    );
    p.commands.insert_resource(maze.clone());

    // 出生点与出口点
    let spawn_pos = pick_spawn_world(&maze, Vec3::new(0.0, 0.0, 0.0));
    p.spawn.world_pos = Vec3::new(spawn_pos.x, 1.0, spawn_pos.z);
    player_t.translation = p.spawn.world_pos;
    let exit_pos = pick_exit_world(&maze, p.spawn.world_pos, &mut rng);
    p.exit_pos_res.world_pos = Vec3::new(exit_pos.x, 0.6, exit_pos.z);

    // 出口（显眼发光柱）
    p.commands.spawn((
        PbrBundle {
            mesh: p.meshes.add(Cuboid::new(1.2, 3.6, 1.2)),
            material: p.materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 0.3),
                emissive: LinearRgba::new(0.12, 0.45, 0.16, 1.0),
                ..default()
            }),
            transform: Transform::from_xyz(p.exit_pos_res.world_pos.x, 1.8, p.exit_pos_res.world_pos.z),
            ..default()
        },
        Exit,
        Collider::cuboid(0.8, 1.8, 0.8),
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
        RigidBody::Fixed,
        WorldEntity,
    ));
    p.commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 22_000.0,
                range: 18.0,
                color: Color::srgb(0.2, 0.9, 0.3),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(p.exit_pos_res.world_pos.x, 2.6, p.exit_pos_res.world_pos.z),
            ..default()
        },
        WorldEntity,
    ));

    // 本层道具/钥匙/锁门由“随机锁门”逻辑统一生成

    // 敌人（序列帧怪物）
    let enemy_count = (p.floor.floor * 2).min(20);
    let enemy_tex: Handle<Image> = p.assets.load("textures/enemy_cc0.png");
    let enemy_mat_proto = StandardMaterial {
        base_color_texture: Some(enemy_tex),
        base_color: Color::srgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    };
    let mut enemy_placed: Vec<Vec3> = Vec::with_capacity(enemy_count as usize);
    let mut enemy_tries = 0;
    while enemy_placed.len() < enemy_count as usize && enemy_tries < enemy_count as usize * 60 {
        enemy_tries += 1;
        let _x = rng.gen_range(-4.6..4.6);
        let pos = random_walkable_tile_world(&maze, &mut rng);
        let player_safe = Vec3::new(0.0, 1.0, 6.0);
        if pos.distance(Vec3::new(player_safe.x, pos.y, player_safe.z)) < 8.0 {
            continue;
        }
        if enemy_placed.iter().any(|q| q.distance(pos) < 2.0) {
            continue;
        }
        enemy_placed.push(pos);

        let enemy_mat = p.materials.add(enemy_mat_proto.clone());
        let enemy_mesh = p.meshes.add(make_billboard_mesh(2.1, 2.9, enemy_frame_uv(0)));
        p.commands.spawn((
            PbrBundle {
                mesh: enemy_mesh,
                material: enemy_mat,
                transform: Transform::from_xyz(pos.x, 0.85, pos.z),
                ..default()
            },
            Enemy,
            Health { hp: 30 },
            EnemyAi {
                speed: 1.05,
                attack_range: 1.15,
                attack_cooldown_s: 1.6,
                damage: 3,
                cooldown_left_s: 0.0,
            },
            EnemyAnim {
                walk_seq: ENEMY_WALK_SEQ.to_vec(),
                hit_seq: ENEMY_HIT_SEQ.to_vec(),
                die_seq: ENEMY_DIE_SEQ.to_vec(),
                state: EnemyAnimState::Walk,
                frame_i: 0,
                hit_left_s: 0.0,
                death_hold_left_s: 0.0,
                fps: 10.0,
                acc: 0.0,
            },
            RigidBody::Dynamic,
            GravityScale(0.0),
            Ccd::enabled(),
            Damping {
                linear_damping: 4.0,
                angular_damping: 8.0,
            },
            Velocity::zero(),
            // 命中判定加宽一点（含手臂区域）
            Collider::capsule_y(0.55, 0.42),
            LockedAxes::ROTATION_LOCKED,
            WorldEntity,
        ));
    }
}

fn generate_rooms_and_corridors_tiles(
    w_cells: u32,
    h_cells: u32,
    rng: &mut StdRng,
) -> (u32, u32, Vec<bool>) {
    // 目标：更像早期FPS的“房间+走廊”，而不是纯迷宫。
    // tile 网格：墙=true，路=false
    let tw = w_cells * 2 + 1;
    let th = h_cells * 2 + 1;
    let mut tiles = vec![true; (tw * th) as usize];
    let idx = |x: u32, y: u32| (y * tw + x) as usize;

    // carve helper
    let mut carve_rect = |x0: u32, y0: u32, x1: u32, y1: u32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                tiles[idx(x, y)] = false;
            }
        }
    };

    // 房间列表（tile 坐标）
    let mut rooms: Vec<(u32, u32, u32, u32)> = Vec::new();
    let room_count = 6 + (rng.gen_range(0..3));
    for _ in 0..room_count {
        let rw = rng.gen_range(3..6) * 2 + 1; // odd
        let rh = rng.gen_range(3..6) * 2 + 1;
        let x0 = rng.gen_range(1..(tw - rw - 1));
        let y0 = rng.gen_range(1..(th - rh - 1));
        let x0 = if x0 % 2 == 0 { x0 + 1 } else { x0 };
        let y0 = if y0 % 2 == 0 { y0 + 1 } else { y0 };
        let x1 = x0 + rw;
        let y1 = y0 + rh;

        // 简单防重叠（留 1 格缓冲）
        let overlaps = rooms.iter().any(|(ax0, ay0, ax1, ay1)| {
            !(x1 + 1 < *ax0 || *ax1 + 1 < x0 || y1 + 1 < *ay0 || *ay1 + 1 < y0)
        });
        if overlaps {
            continue;
        }

        carve_rect(x0, y0, x1, y1);
        rooms.push((x0, y0, x1, y1));
    }

    // 如果房间太少，退化为一个大房间
    if rooms.len() < 2 {
        carve_rect(3, 3, tw - 4, th - 4);
        return (tw, th, tiles);
    }

    // 连接房间：中心点 Manhattan corridor（L 形）
    let center = |r: (u32, u32, u32, u32)| ((r.0 + r.2) / 2, (r.1 + r.3) / 2);
    for i in 1..rooms.len() {
        let (ax, ay) = center(rooms[i - 1]);
        let (bx, by) = center(rooms[i]);
        if rng.gen_bool(0.5) {
            carve_rect(ax.min(bx), ay, ax.max(bx), ay);
            carve_rect(bx, ay.min(by), bx, ay.max(by));
        } else {
            carve_rect(ax, ay.min(by), ax, ay.max(by));
            carve_rect(ax.min(bx), by, ax.max(bx), by);
        }
    }

    (tw, th, tiles)
}

fn spawn_maze_walls(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    maze: &MazeData,
) {
    let wall_tex: Handle<Image> = assets.load("textures/wall_cc0.png");
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        base_color: Color::srgb(0.85, 0.85, 0.88),
        emissive: LinearRgba::new(0.02, 0.02, 0.025, 1.0),
        perceptual_roughness: 0.98,
        ..default()
    });

    let wall_h = 2.4;
    let thick = 0.22;
    let mesh = meshes.add(Cuboid::new(maze.tile_size_world, wall_h, maze.tile_size_world));

    for y in 0..maze.tiles_h {
        for x in 0..maze.tiles_w {
            let is_wall = maze.tiles[(y * maze.tiles_w + x) as usize];
            if !is_wall {
                continue;
            }
            let wx = maze.origin_world.x + x as f32 * maze.tile_size_world;
            let wz = maze.origin_world.z + y as f32 * maze.tile_size_world;
            // 只在墙格上放方块墙（简化）。后续可优化成条带减少实体数量。
            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material: wall_mat.clone(),
                    transform: Transform::from_xyz(wx, wall_h * 0.5, wz),
                    ..default()
                },
                Collider::cuboid(maze.tile_size_world * 0.5, wall_h * 0.5, maze.tile_size_world * 0.5),
                RigidBody::Fixed,
                WorldEntity,
            ));
        }
    }

    let _ = thick;
}

fn random_walkable_tile_world(maze: &MazeData, rng: &mut StdRng) -> Vec3 {
    for _ in 0..2000 {
        let x = rng.gen_range(1..(maze.tiles_w - 1));
        let y = rng.gen_range(1..(maze.tiles_h - 1));
        let i = (y * maze.tiles_w + x) as usize;
        if maze.tiles[i] {
            continue;
        }
        let wx = maze.origin_world.x + x as f32 * maze.tile_size_world;
        let wz = maze.origin_world.z + y as f32 * maze.tile_size_world;
        return Vec3::new(wx, 0.0, wz);
    }
    Vec3::new(0.0, 0.0, 10.0)
}

fn maze_tile_to_world_center(maze: &MazeData, x: u32, y: u32, y_height: f32) -> Vec3 {
    Vec3::new(
        maze.origin_world.x + x as f32 * maze.tile_size_world,
        y_height,
        maze.origin_world.z + y as f32 * maze.tile_size_world,
    )
}

fn maze_world_to_tile(maze: &MazeData, w: Vec3) -> Option<(u32, u32)> {
    let x = ((w.x - maze.origin_world.x) / maze.tile_size_world).round() as i32;
    let y = ((w.z - maze.origin_world.z) / maze.tile_size_world).round() as i32;
    if x < 0 || y < 0 || x >= maze.tiles_w as i32 || y >= maze.tiles_h as i32 {
        return None;
    }
    Some((x as u32, y as u32))
}

fn maze_is_open(maze: &MazeData, x: u32, y: u32) -> bool {
    let i = (y * maze.tiles_w + x) as usize;
    !maze.tiles[i]
}

fn maze_bfs_dist(maze: &MazeData, start: (u32, u32)) -> Vec<i32> {
    let mut dist = vec![-1i32; (maze.tiles_w * maze.tiles_h) as usize];
    let idx = |x: u32, y: u32| (y * maze.tiles_w + x) as usize;
    if !maze_is_open(maze, start.0, start.1) {
        return dist;
    }
    let mut q: std::collections::VecDeque<(u32, u32)> = std::collections::VecDeque::new();
    dist[idx(start.0, start.1)] = 0;
    q.push_back(start);
    while let Some((x, y)) = q.pop_front() {
        let d0 = dist[idx(x, y)];
        let mut push = |nx: i32, ny: i32| {
            if nx < 0 || ny < 0 {
                return;
            }
            let nx = nx as u32;
            let ny = ny as u32;
            if nx >= maze.tiles_w || ny >= maze.tiles_h {
                return;
            }
            if !maze_is_open(maze, nx, ny) {
                return;
            }
            let ii = idx(nx, ny);
            if dist[ii] >= 0 {
                return;
            }
            dist[ii] = d0 + 1;
            q.push_back((nx, ny));
        };
        push(x as i32 - 1, y as i32);
        push(x as i32 + 1, y as i32);
        push(x as i32, y as i32 - 1);
        push(x as i32, y as i32 + 1);
    }
    dist
}

fn pick_locked_door_tile(
    maze: &MazeData,
    dist: &[i32],
    exit_tile: (u32, u32),
) -> Option<((u32, u32), DoorAxis, i32)> {
    let idx = |x: u32, y: u32| (y * maze.tiles_w + x) as usize;
    let exit_d = dist[idx(exit_tile.0, exit_tile.1)];
    if exit_d <= 0 {
        return None;
    }
    let mut best: Option<((u32, u32), DoorAxis, i32)> = None;
    for y in 1..(maze.tiles_h - 1) {
        for x in 1..(maze.tiles_w - 1) {
            if !maze_is_open(maze, x, y) {
                continue;
            }
            let d = dist[idx(x, y)];
            if d < 10 || d > exit_d - 6 {
                continue;
            }
            let open_l = maze_is_open(maze, x - 1, y);
            let open_r = maze_is_open(maze, x + 1, y);
            let open_u = maze_is_open(maze, x, y - 1);
            let open_d = maze_is_open(maze, x, y + 1);
            // corridor chokepoint: only two opposite neighbors open
            let axis = if open_u && open_d && !open_l && !open_r {
                Some(DoorAxis::Ns)
            } else if open_l && open_r && !open_u && !open_d {
                Some(DoorAxis::Ew)
            } else {
                None
            };
            let Some(axis) = axis else { continue };
            match best {
                None => best = Some(((x, y), axis, d)),
                Some((_, _, bd)) if d > bd => best = Some(((x, y), axis, d)),
                _ => {}
            }
        }
    }
    best
}

fn pick_tile_with_dist_range(
    maze: &MazeData,
    dist: &[i32],
    min_d: i32,
    max_d: i32,
    rng: &mut StdRng,
) -> Option<(u32, u32)> {
    let idx = |x: u32, y: u32| (y * maze.tiles_w + x) as usize;
    let mut candidates: Vec<(u32, u32)> = Vec::new();
    for y in 1..(maze.tiles_h - 1) {
        for x in 1..(maze.tiles_w - 1) {
            if !maze_is_open(maze, x, y) {
                continue;
            }
            let d = dist[idx(x, y)];
            if d >= min_d && d <= max_d {
                candidates.push((x, y));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.gen_range(0..candidates.len())])
}

fn spawn_locked_door(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    maze: &MazeData,
    tile: (u32, u32),
    axis: DoorAxis,
    locked: bool,
) {
    let door_tex: Handle<Image> = assets.load("textures/door_cc0.png");
    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(door_tex),
        base_color: Color::srgb(0.82, 0.82, 0.85),
        emissive: LinearRgba::new(0.02, 0.02, 0.025, 1.0),
        perceptual_roughness: 0.95,
        ..default()
    });
    let h = 2.2;
    let (sx, sz) = match axis {
        DoorAxis::Ns => (maze.tile_size_world * 0.95, 0.25),
        DoorAxis::Ew => (0.25, maze.tile_size_world * 0.95),
    };
    let pos = maze_tile_to_world_center(maze, tile.0, tile.1, h * 0.5);
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(sx, h, sz)),
            material: door_mat,
            transform: Transform::from_translation(pos),
            ..default()
        },
        Door { locked },
        Collider::cuboid(sx * 0.5, h * 0.5, sz * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));
}

fn pick_spawn_world(maze: &MazeData, prefer_world: Vec3) -> Vec3 {
    let to_tile = |w: Vec3| {
        let x = ((w.x - maze.origin_world.x) / maze.tile_size_world).round() as i32;
        let y = ((w.z - maze.origin_world.z) / maze.tile_size_world).round() as i32;
        (x, y)
    };
    let (tx0, ty0) = to_tile(prefer_world);

    let in_bounds = |x: i32, y: i32| x >= 1 && y >= 1 && x < (maze.tiles_w as i32 - 1) && y < (maze.tiles_h as i32 - 1);
    let is_walk = |x: i32, y: i32| {
        if !in_bounds(x, y) {
            return false;
        }
        let i = (y as u32 * maze.tiles_w + x as u32) as usize;
        !maze.tiles[i]
    };
    let to_world = |x: i32, y: i32| {
        Vec3::new(
            maze.origin_world.x + x as f32 * maze.tile_size_world,
            0.0,
            maze.origin_world.z + y as f32 * maze.tile_size_world,
        )
    };

    if is_walk(tx0, ty0) {
        return to_world(tx0, ty0);
    }
    for r in 1..120 {
        for dx in -r..=r {
            let x = tx0 + dx;
            let y1 = ty0 - r;
            if is_walk(x, y1) {
                return to_world(x, y1);
            }
            let y2 = ty0 + r;
            if is_walk(x, y2) {
                return to_world(x, y2);
            }
        }
        for dy in (-r + 1)..=(r - 1) {
            let y = ty0 + dy;
            let x1 = tx0 - r;
            if is_walk(x1, y) {
                return to_world(x1, y);
            }
            let x2 = tx0 + r;
            if is_walk(x2, y) {
                return to_world(x2, y);
            }
        }
    }

    Vec3::new(0.0, 0.0, 0.0)
}

fn pick_exit_world(maze: &MazeData, spawn_world: Vec3, rng: &mut StdRng) -> Vec3 {
    // 从多个可走格采样，选离出生点最远的作为出口（稳定又足够快）
    let mut best = Vec3::new(0.0, 0.0, 0.0);
    let mut best_d = -1.0f32;
    for _ in 0..600 {
        let p = random_walkable_tile_world(maze, rng);
        let d = (p.x - spawn_world.x).powi(2) + (p.z - spawn_world.z).powi(2);
        if d > best_d {
            best_d = d;
            best = p;
        }
    }
    best
}

fn toggle_minimap(
    keys: Res<ButtonInput<KeyCode>>,
    mut s: ResMut<MinimapState>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        s.visible = !s.visible;
    }
}

fn update_minimap(
    mut commands: Commands,
    s: Res<MinimapState>,
    maze: Option<Res<MazeData>>,
    ui: Query<Entity, With<MinimapUi>>,
    mut pool: ResMut<MinimapEnemyPool>,
    mut styles: ParamSet<(
        Query<&mut Style, With<MinimapPlayer>>,
        Query<&mut Style, With<MinimapEnemy>>,
    )>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    enemies: Query<&GlobalTransform, With<Enemy>>,
) {
    let Some(maze) = maze else { return };

    // 创建/销毁 UI（避免每帧大量更新）
    if !s.visible {
        for e in &ui {
            if let Some(ec) = commands.get_entity(e) {
                ec.despawn_recursive();
            }
        }
        pool.markers.clear();
        return;
    }
    if ui.iter().next().is_none() {
        let tile_px = 6.0;
        let w = maze.tiles_w as f32 * tile_px;
        let h = maze.tiles_h as f32 * tile_px;
        let mut markers: Vec<Entity> = Vec::new();

        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        right: Val::Px(14.0),
                        top: Val::Px(12.0),
                        width: Val::Px(w + 10.0),
                        height: Val::Px(h + 10.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    ..default()
                },
                MinimapUi,
            ))
            .with_children(|p| {
                // tiles
                for y in 0..maze.tiles_h {
                    for x in 0..maze.tiles_w {
                        let idx = (y * maze.tiles_w + x) as usize;
                        let is_wall = maze.tiles[idx];
                        p.spawn((
                            NodeBundle {
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(x as f32 * tile_px),
                                    top: Val::Px(y as f32 * tile_px),
                                    width: Val::Px(tile_px),
                                    height: Val::Px(tile_px),
                                    ..default()
                                },
                                background_color: BackgroundColor(if is_wall {
                                    Color::srgba(0.7, 0.7, 0.75, 0.55)
                                } else {
                                    Color::srgba(0.0, 0.0, 0.0, 0.0)
                                }),
                                ..default()
                            },
                            MinimapTile { idx },
                        ));
                    }
                }

                // player marker
                p.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Px(tile_px),
                            height: Val::Px(tile_px),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgb(0.2, 0.8, 1.0)),
                        ..default()
                    },
                    MinimapPlayer,
                ));

                // enemy markers：固定池，避免每帧 spawn/despawn 导致 “parent 不存在” 崩溃
                const MAX_ENEMY_MARKERS: usize = 32;
                for _ in 0..MAX_ENEMY_MARKERS {
                    let id = p
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    display: Display::None,
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(tile_px),
                                    height: Val::Px(tile_px),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgb(1.0, 0.2, 0.2)),
                                ..default()
                            },
                            MinimapEnemy,
                        ))
                        .id();
                    markers.push(id);
                }
            });
        pool.markers = markers;
    }

    let Some(root) = ui.iter().next() else { return };
    let Ok(p) = player.get_single() else { return };

    let tile_px = 6.0;
    let to_tile = |w: Vec3| {
        let x = ((w.x - maze.origin_world.x) / maze.tile_size_world).round();
        let y = ((w.z - maze.origin_world.z) / maze.tile_size_world).round();
        (x, y)
    };

    // 更新玩家 marker
    if let Ok(mut st) = styles.p0().get_single_mut() {
        let (tx, ty) = to_tile(p.translation());
        st.left = Val::Px(tx * tile_px);
        st.top = Val::Px(ty * tile_px);
    }

    let _ = root; // root 仅用于维持 UI 存在；enemy markers 在固定池内更新

    // 更新敌人 markers（只改 Style，不再 spawn/despawn）
    let mut i = 0usize;
    for e in &enemies {
        if i >= pool.markers.len() {
            break;
        }
        let marker_e = pool.markers[i];
        if let Ok(mut st) = styles.p1().get_mut(marker_e) {
            let (tx, ty) = to_tile(e.translation());
            st.display = Display::Flex;
            st.left = Val::Px(tx * tile_px);
            st.top = Val::Px(ty * tile_px);
        }
        i += 1;
    }
    // 隐藏多余 markers
    for j in i..pool.markers.len() {
        let marker_e = pool.markers[j];
        if let Ok(mut st) = styles.p1().get_mut(marker_e) {
            st.display = Display::None;
        }
    }
}

fn fps_look(
    mut q: Query<(&mut Transform, &mut FpsLook), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut t, mut look)) = q.get_single_mut() else {
        return;
    };

    let kb_yaw = (keys.pressed(KeyCode::ArrowLeft) as i32 - keys.pressed(KeyCode::ArrowRight) as i32) as f32;
    let kb_pitch = (keys.pressed(KeyCode::ArrowDown) as i32 - keys.pressed(KeyCode::ArrowUp) as i32) as f32;
    if kb_yaw != 0.0 || kb_pitch != 0.0 {
        // 键盘视角速度（rad/s）。独立于 mouse sensitivity，保证能用。
        let yaw_speed = 0.9;
        let pitch_speed = 1.2;
        let dt = time.delta_seconds().min(0.05);
        look.yaw += kb_yaw * yaw_speed * dt;
        look.pitch = (look.pitch + kb_pitch * pitch_speed * dt).clamp(-1.54, 1.54);
    }

    t.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
}

fn fps_move(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_body: Query<(&Transform, &mut Velocity), With<PlayerBody>>,
    camera: Query<&Transform, (With<Player>, Without<PlayerBody>)>,
) {
    let Ok((body_t, mut vel)) = player_body.get_single_mut() else {
        return;
    };
    let Ok(cam_t) = camera.get_single() else { return };

    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        wish.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        wish.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish.x += 1.0;
    }
    if wish == Vec3::ZERO {
        // 不输入时直接把水平速度归零，避免被碰撞/推挤“带着跑”
        vel.linvel.x = 0.0;
        vel.linvel.z = 0.0;
        return;
    }

    // 行走/冲刺速度（m/s）
    const WALK_SPEED: f32 = 20.0;
    const SPRINT_SPEED: f32 = 30.0;
    let speed = if keys.pressed(KeyCode::ShiftLeft) { SPRINT_SPEED } else { WALK_SPEED };
    let forward = cam_t.forward();
    let right = cam_t.right();
    let dir3 = right * wish.x + forward * wish.z;
    let dir = Vec3::new(dir3.x, 0.0, dir3.z).normalize_or_zero();

    // 只控制水平面移动；垂直由重力/碰撞决定
    let target = dir * speed;
    vel.linvel.x = target.x;
    vel.linvel.z = target.z;

    // 防止“翻滚/漂移”导致的高度异常：当跌出世界时重置
    if body_t.translation.y < -50.0 {
        vel.linvel = Vec3::ZERO;
    }
}

#[derive(SystemParam)]
struct ShootParams<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    time: Res<'w, Time>,
    rapier: Res<'w, RapierContext>,
    camera: Query<'w, 's, &'static GlobalTransform, With<Player>>,
    muzzle: Query<'w, 's, &'static GlobalTransform, With<GunMuzzle>>,
    player_body: Query<'w, 's, Entity, With<PlayerBody>>,
    enemies: Query<'w, 's, (), With<Enemy>>,
    world: Query<'w, 's, (), With<WorldEntity>>,
    health: Query<'w, 's, &'static mut Health, With<Enemy>>,
    enemy_mats: Query<'w, 's, &'static Handle<StandardMaterial>, With<Enemy>>,
    enemy_anim: Query<'w, 's, &'static mut EnemyAnim, With<Enemy>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    commands: Commands<'w, 's>,
    fire: ResMut<'w, FireState>,
    loadout: ResMut<'w, PlayerLoadout>,
    audio: Res<'w, AudioAssets>,
    hermes: EventWriter<'w, HermesEvent>,
}

fn shoot(
    mut p: ShootParams,
) {
    let dt = p.time.delta_seconds().min(0.05);
    p.fire.cooldown_s = (p.fire.cooldown_s - dt).max(0.0);
    if !p.keys.pressed(KeyCode::Space) {
        return;
    }
    if p.fire.cooldown_s > 0.0 {
        return;
    }
    // 手枪/霰弹枪不同射速（秒/发）
    p.fire.cooldown_s = match p.loadout.weapon {
        WeaponType::Pistol => 0.12,
        WeaponType::Shotgun => 0.42,
    };

    p.commands.spawn(AudioBundle {
        source: p.audio.shoot.clone(),
        settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.65)),
    });
    let Ok(cam) = p.camera.get_single() else { return };

    // 用摄像机中心作为射线起点，保证与准星一致（不受枪口模型偏斜影响）
    let origin = cam.translation();
    let dir = *cam.forward();
    let player_e = p.player_body.get_single().ok();

    // 只允许命中 Enemy，避免先打到墙/柱子/地板导致“打不中”
    let only_enemy = |e| p.enemies.contains(e);
    let filter = QueryFilter::default().predicate(&only_enemy);
    let mut did_hit = false;

    let (pellets, dmg, max_dist) = match p.loadout.weapon {
        WeaponType::Pistol => (1, 12, 100.0),
        WeaponType::Shotgun => {
            if p.loadout.shotgun_ammo <= 0 {
                p.hermes.send(HermesEvent {
                    topic: HermesTopic::Gameplay,
                    from: AgentRole::GameplayProgrammer,
                    message: "霰弹枪：无弹药".to_string(),
                });
                return;
            }
            p.loadout.shotgun_ammo -= 1;
            (7, 6, 55.0)
        }
    };

    let mut world_decal_budget = 3usize; // 霰弹枪一枪最多落 3 个弹坑，避免刷屏卡顿
    let mut did_world_impact = false;

    for i in 0..pellets {
        let mut d = dir;
        if pellets > 1 {
            // 简单散射：固定模式 + 少量偏移（避免引入更多随机状态）
            let a = match i {
                0 => (0.0, 0.0),
                1 => (0.02, 0.0),
                2 => (-0.02, 0.0),
                3 => (0.0, 0.02),
                4 => (0.0, -0.02),
                5 => (0.02, 0.02),
                _ => (-0.02, -0.02),
            };
            d = (d + cam.right() * a.0 + cam.up() * a.1).normalize_or_zero();
        }

        if let Some((hit_entity, toi)) = p.rapier.cast_ray(origin, d, max_dist, true, filter) {
            did_hit = true;
            let hit_pos = origin + d * toi;
            spawn_tracer(&mut p.commands, &mut p.meshes, &mut p.materials, origin, hit_pos);
            spawn_hit_spark(&mut p.commands, &mut p.meshes, &mut p.materials, hit_pos);

            if let Ok(mut hp) = p.health.get_mut(hit_entity) {
                hp.hp -= dmg;
                if let Ok(mat_handle) = p.enemy_mats.get(hit_entity) {
                    if let Some(m) = p.materials.get_mut(mat_handle) {
                        m.base_color = Color::srgb(1.0, 0.25, 0.25);
                        m.emissive = LinearRgba::new(0.25, 0.02, 0.02, 1.0);
                    }
                }
                // 换层/清理时 Enemy 可能在本帧被 despawn：插入前先确认 entity 仍存在
                if let Some(mut ec) = p.commands.get_entity(hit_entity) {
                    ec.insert(HitFlash { left_s: 0.12 });
                }
                if let Ok(mut anim) = p.enemy_anim.get_mut(hit_entity) {
                    if anim.state != EnemyAnimState::Die {
                        if hp.hp <= 0 {
                            hp.hp = 0;
                            anim.state = EnemyAnimState::Die;
                            anim.frame_i = 0;
                            anim.acc = 0.0;
                            anim.hit_left_s = 0.0;
                            anim.death_hold_left_s = 0.0;
                        } else {
                            anim.state = EnemyAnimState::Hit;
                            anim.frame_i = 0;
                            anim.acc = 0.0;
                            anim.hit_left_s = 0.18;
                        }
                    }
                }
            }
        }

        // 没打到敌人时，也检测是否命中墙/地面（需要法线来贴弹坑）
        let mut world_filter = QueryFilter::default().exclude_sensors();
        if let Some(pe) = player_e {
            world_filter = world_filter.exclude_collider(pe).exclude_rigid_body(pe);
        }
        // 关键：弹坑只允许打在“世界几何体”上，不能打在敌人身上
        let only_world = |e| p.world.contains(e);
        world_filter = world_filter.predicate(&only_world);
        if let Some((_e, hit)) = p.rapier.cast_ray_and_get_normal(origin, d, 120.0, true, world_filter) {
            did_world_impact = true;
            let hit_pos = origin + d * hit.time_of_impact;
            spawn_tracer(&mut p.commands, &mut p.meshes, &mut p.materials, origin, hit_pos);
            spawn_hit_spark(&mut p.commands, &mut p.meshes, &mut p.materials, hit_pos);
            if world_decal_budget > 0 {
                world_decal_budget -= 1;
                spawn_bullet_hole(&mut p.commands, &mut p.meshes, &mut p.materials, hit_pos, hit.normal);
            }
        }
    }

    // 完全没命中任何东西时，也显示一段固定长度曳光，方便反馈
    if !did_hit && !did_world_impact {
        spawn_tracer(&mut p.commands, &mut p.meshes, &mut p.materials, origin, origin + dir * 22.0);
    }

    if did_world_impact && !did_hit {
        p.commands.spawn(AudioBundle {
            source: p.audio.impact.clone(),
            settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.75)),
        });
    }

    if did_hit {
        // 命中怪物音效随机
        let idx = (rand::random::<u32>() % 5) as usize;
        p.commands.spawn(AudioBundle {
            source: p.audio.hit_variants[idx].clone(),
            settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.95)),
        });
    }
}

fn spawn_bullet_hole(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    normal: Vec3,
) {
    // 一个很薄的“贴花”方片，稍微沿法线抬起避免 z-fighting
    let size = 0.16;
    let uv = (Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
    let rot = Quat::from_rotation_arc(Vec3::Z, normal.normalize_or_zero());
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(make_billboard_mesh(size, size, uv)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.05, 0.05, 0.05, 0.85),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_translation(pos + normal.normalize_or_zero() * 0.01)
                .with_rotation(rot),
            ..default()
        },
        TimedDespawn { left_s: 10.0 },
    ));
}

fn make_billboard_mesh(width: f32, height: f32, uv: (Vec2, Vec2)) -> Mesh {
    let (uv0, uv1) = uv;
    let hw = width * 0.5;
    let hh = height * 0.5;

    let positions = vec![
        [-hw, -hh, 0.0],
        [ hw, -hh, 0.0],
        [ hw,  hh, 0.0],
        [-hw,  hh, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![
        [uv0.x, uv0.y],
        [uv1.x, uv0.y],
        [uv1.x, uv1.y],
        [uv0.x, uv1.y],
    ];
    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn tick_enemy_anim(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    cfg: Res<EnemySpriteConfig>,
    mut q: Query<(Entity, &mut EnemyAnim, &Handle<Mesh>), (With<Enemy>, Without<Boss>)>,
) {
    let dt = time.delta_seconds().min(0.05);
    for (e, mut anim, mesh) in &mut q {
        anim.acc += dt;
        let frame_time = 1.0 / anim.fps.max(1.0);

        // 死亡：播完序列后停留 3 秒再消失
        if anim.state == EnemyAnimState::Die {
            if anim.die_seq.is_empty() {
                continue;
            }
            // 已经到最后一帧：开始计时
            if anim.frame_i >= anim.die_seq.len().saturating_sub(1) {
                anim.death_hold_left_s = (anim.death_hold_left_s - dt).max(0.0);
                if anim.death_hold_left_s <= 0.0 {
                    commands.entity(e).despawn_recursive();
                }
            // 始终保持最后一帧
            let frame = *anim.die_seq.last().unwrap();
                let (mut uv0, mut uv1) = enemy_frame_uv(frame);
                if cfg.v_flip {
                    (uv0, uv1) = (Vec2::new(uv0.x, uv1.y), Vec2::new(uv1.x, uv0.y));
                }
                if let Some(m) = meshes.get_mut(mesh) {
                    let uvs = vec![
                        [uv0.x, uv0.y],
                        [uv1.x, uv0.y],
                        [uv1.x, uv1.y],
                        [uv0.x, uv1.y],
                    ];
                    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                }
                continue;
            }
            if anim.acc < frame_time {
                continue;
            }
            anim.acc -= frame_time;
            anim.frame_i += 1;
            let frame = anim.die_seq[anim.frame_i.min(anim.die_seq.len() - 1)];
            // 播到最后一帧后开始停留
            if anim.frame_i >= anim.die_seq.len().saturating_sub(1) {
                anim.death_hold_left_s = 3.0;
            }
            let (mut uv0, mut uv1) = enemy_frame_uv(frame);
            if cfg.v_flip {
                (uv0, uv1) = (Vec2::new(uv0.x, uv1.y), Vec2::new(uv1.x, uv0.y));
            }
            if let Some(m) = meshes.get_mut(mesh) {
                let uvs = vec![
                    [uv0.x, uv0.y],
                    [uv1.x, uv0.y],
                    [uv1.x, uv1.y],
                    [uv0.x, uv1.y],
                ];
                m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            }
            continue;
        }

        // 受击：短暂播放一轮，然后回走路
        if anim.state == EnemyAnimState::Hit {
            anim.hit_left_s = (anim.hit_left_s - dt).max(0.0);
            if anim.hit_seq.is_empty() {
                anim.state = EnemyAnimState::Walk;
            }
        }

        let (seq_len, frame) = match anim.state {
            EnemyAnimState::Walk => {
                if anim.walk_seq.is_empty() {
                    continue;
                }
                let len = anim.walk_seq.len();
                (len, anim.walk_seq[anim.frame_i % len])
            }
            EnemyAnimState::Hit => {
                if anim.hit_seq.is_empty() {
                    continue;
                }
                let len = anim.hit_seq.len();
                (len, anim.hit_seq[anim.frame_i % len])
            }
            EnemyAnimState::Die => continue, // 上面已处理
        };
        if seq_len == 0 {
            continue;
        }
        if anim.acc < frame_time {
            continue;
        }
        anim.acc -= frame_time;
        anim.frame_i = (anim.frame_i + 1) % seq_len;
        let frame = match anim.state {
            EnemyAnimState::Walk => anim.walk_seq[anim.frame_i],
            EnemyAnimState::Hit => anim.hit_seq[anim.frame_i],
            EnemyAnimState::Die => frame,
        };
        if anim.state == EnemyAnimState::Hit && anim.hit_left_s <= 0.0 {
            anim.state = EnemyAnimState::Walk;
            anim.frame_i = 0;
        }
        let (mut uv0, mut uv1) = enemy_frame_uv(frame);
        if cfg.v_flip {
            (uv0, uv1) = (Vec2::new(uv0.x, uv1.y), Vec2::new(uv1.x, uv0.y));
        }

        if let Some(m) = meshes.get_mut(mesh) {
            let uvs = vec![
                [uv0.x, uv0.y],
                [uv1.x, uv0.y],
                [uv1.x, uv1.y],
                [uv0.x, uv1.y],
            ];
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
    }
}

fn spawn_tracer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    from: Vec3,
    to: Vec3,
) {
    let mid = (from + to) * 0.5;
    let dir = to - from;
    let len = dir.length().max(0.01);
    let rot = Quat::from_rotation_arc(Vec3::Z, dir.normalize_or_zero());
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(0.05, 0.05, len)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.95, 0.6),
                emissive: LinearRgba::new(0.7, 0.55, 0.1, 1.0),
                unlit: true,
                ..default()
            }),
            transform: Transform::from_translation(mid).with_rotation(rot),
            ..default()
        },
        TimedDespawn { left_s: 0.12 },
    ));
}

fn spawn_hit_spark(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(0.18)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.4),
                emissive: LinearRgba::new(0.9, 0.5, 0.1, 1.0),
                unlit: true,
                ..default()
            }),
            transform: Transform::from_translation(pos),
            ..default()
        },
        TimedDespawn { left_s: 0.22 },
    ));
}

fn enemy_ai(
    time: Res<Time>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    mut stats: ResMut<PlayerStats>,
    mut enemies: Query<(&mut Transform, &mut Velocity, &mut EnemyAi, &Health), (With<Enemy>, Without<Boss>)>,
    mut hermes: EventWriter<HermesEvent>,
    mut next: ResMut<NextState<AppState>>,
    audio: Res<AudioAssets>,
    mut commands: Commands,
) {
    let Ok(p) = player.get_single() else { return };
    let ppos = p.translation();
    let dt = time.delta_seconds().min(0.05);

    for (mut t, mut vel, mut ai, hp) in &mut enemies {
        if hp.hp <= 0 {
            vel.linvel = Vec3::ZERO;
            continue;
        }

        ai.cooldown_left_s = (ai.cooldown_left_s - dt).max(0.0);

        let epos = t.translation;
        let to = ppos - epos;
        let dist = to.length();
        let dir = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();

        vel.linvel = dir * ai.speed;
        if dir != Vec3::ZERO {
            // 让贴图“正面朝向玩家”。我们的薄片正面朝 +Z，因此需要额外 +PI。
            t.rotation = Quat::from_rotation_y(f32::atan2(dir.x, dir.z) + std::f32::consts::PI);
        }

        if dist <= ai.attack_range && ai.cooldown_left_s <= 0.0 {
            ai.cooldown_left_s = ai.attack_cooldown_s;
            stats.hp -= ai.damage;
            commands.spawn(AudioBundle {
                source: audio.monster.clone(),
                settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.7)),
            });
            hermes.send(HermesEvent {
                topic: HermesTopic::Gameplay,
                from: AgentRole::GameplayProgrammer,
                message: format!("玩家受伤：-{}（HP {}）", ai.damage, stats.hp),
            });
            if stats.hp <= 0 {
                hermes.send(HermesEvent {
                    topic: HermesTopic::ProducerGate,
                    from: AgentRole::Producer,
                    message: "玩家死亡，返回主菜单".to_string(),
                });
                stats.hp = 100;
                next.set(AppState::MainMenu);
                break;
            }
        }
    }
}

fn tick_timed_despawn(
    time: Res<Time>,
    mut commands: Commands,
    mut timed: Query<(Entity, &mut TimedDespawn)>,
    mut flashes: Query<(Entity, &mut HitFlash, &Handle<StandardMaterial>), With<Enemy>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_seconds().min(0.05);

    for (e, mut t) in &mut timed {
        t.left_s -= dt;
        if t.left_s <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }

    for (e, mut f, mat) in &mut flashes {
        f.left_s -= dt;
        if f.left_s <= 0.0 {
            if let Some(m) = materials.get_mut(mat) {
                // 恢复默认颜色（贴图本色）
                m.base_color = Color::srgb(1.0, 1.0, 1.0);
                m.emissive = LinearRgba::new(0.0, 0.0, 0.0, 1.0);
            }
            commands.entity(e).remove::<HitFlash>();
        }
    }
}

fn update_hud(
    stats: Res<PlayerStats>,
    floor: Res<FloorState>,
    loadout: Res<PlayerLoadout>,
    cons: Res<PlayerConsumables>,
    mut q: Query<&mut Text, With<HudText>>,
) {
    if !stats.is_changed() && !floor.is_changed() && !loadout.is_changed() && !cons.is_changed() {
        return;
    }
    let Ok(mut t) = q.get_single_mut() else { return };
    let w = match loadout.weapon {
        WeaponType::Pistol => "Pistol",
        WeaponType::Shotgun => "Shotgun",
    };
    t.sections[0].value = format!(
        "HP: {} | Floor: {} | {} | Shells: {} | 血包: {}",
        stats.hp, floor.floor, w, loadout.shotgun_ammo, cons.medkits
    );
}

fn weapon_switch(keys: Res<ButtonInput<KeyCode>>, mut loadout: ResMut<PlayerLoadout>) {
    if keys.just_pressed(KeyCode::Digit1) {
        loadout.weapon = WeaponType::Pistol;
    }
    if keys.just_pressed(KeyCode::Digit2) && loadout.shotgun_unlocked {
        loadout.weapon = WeaponType::Shotgun;
    }
}

fn pickup_items(
    mut commands: Commands,
    mut inventory: ResMut<Inventory>,
    mut cons: ResMut<PlayerConsumables>,
    mut loadout: ResMut<PlayerLoadout>,
    mut toast: ResMut<PickupToast>,
    audio: Res<AudioAssets>,
    assets: Res<AssetServer>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    items: Query<(Entity, &GlobalTransform, &ItemPickup)>,
    mut hermes: EventWriter<HermesEvent>,
) {
    let Ok(p) = player.get_single() else { return };
    let ppos = p.translation();

    for (e, t, item) in &items {
        if ppos.distance(t.translation()) > 1.2 {
            continue;
        }
        let float_pos = t.translation() + Vec3::Y * 1.25;
        let float_text: Option<String> = match item.kind {
            ItemKind::Key => {
                inventory.has_key = true;
                hermes.send(HermesEvent {
                    topic: HermesTopic::Gameplay,
                    from: AgentRole::GameplayProgrammer,
                    message: "拾取：钥匙".to_string(),
                });
                toast.text = "拾取：钥匙 x1".to_string();
                toast.left_s = 1.4;
                Some("钥匙 x1".to_string())
            }
            ItemKind::Health(v) => {
                // 血包存起来，按键消费
                cons.medkits += 1;
                hermes.send(HermesEvent {
                    topic: HermesTopic::Gameplay,
                    from: AgentRole::GameplayProgrammer,
                    message: format!("拾取：血包 +{}（库存 {}）", v, cons.medkits),
                });
                toast.text = format!("拾取：血包 x1（库存 {}）", cons.medkits);
                toast.left_s = 1.4;
                Some(format!("血包 x1（库存 {}）", cons.medkits))
            }
            ItemKind::Shotgun { ammo } => {
                loadout.shotgun_unlocked = true;
                loadout.shotgun_ammo += ammo;
                hermes.send(HermesEvent {
                    topic: HermesTopic::Gameplay,
                    from: AgentRole::GameplayProgrammer,
                    message: format!("拾取：霰弹枪（Shells +{}）", ammo),
                });
                toast.text = format!("拾取：霰弹枪 x1（Shells +{}）", ammo);
                toast.left_s = 1.4;
                Some(format!("霰弹枪 x1（Shells +{}）", ammo))
            }
            ItemKind::AmmoShells(v) => {
                loadout.shotgun_ammo += v;
                hermes.send(HermesEvent {
                    topic: HermesTopic::Gameplay,
                    from: AgentRole::GameplayProgrammer,
                    message: format!("拾取：霰弹 Shells +{}（{}）", v, loadout.shotgun_ammo),
                });
                toast.text = format!("拾取：霰弹 Shells +{}（{}）", v, loadout.shotgun_ammo);
                toast.left_s = 1.4;
                Some(format!("霰弹 Shells x{}（{}）", v, loadout.shotgun_ammo))
            }
        };
        if let Some(txt) = float_text {
            let font: Handle<Font> = assets.load("fonts/NotoSansCJKsc-Regular.otf");
            commands.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        txt,
                        TextStyle {
                            font,
                            font_size: 26.0,
                            color: Color::srgb(1.0, 0.95, 0.85),
                            ..default()
                        },
                    )
                    .with_justify(JustifyText::Center),
                    transform: Transform::from_translation(float_pos),
                    ..default()
                },
                FloatingPickupText,
                TimedDespawn { left_s: 1.2 },
            ));
        }
        commands.spawn(AudioBundle {
            source: audio.pickup.clone(),
            settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.8)),
        });
        commands.entity(e).despawn_recursive();
    }
}

fn face_floating_pickup_text(
    cam: Query<&GlobalTransform, (With<Player>, Without<FloatingPickupText>)>,
    mut q: Query<&mut Transform, With<FloatingPickupText>>,
) {
    let Ok(cam_t) = cam.get_single() else { return };
    let cam_pos = cam_t.translation();
    for mut t in &mut q {
        // 让文字始终朝向玩家（摄像机）
        t.look_at(cam_pos, Vec3::Y);
        // Text2d 的正面朝 -Z，额外旋转 180° 保证不反向（不同版本可能不需要，但这里保持稳定）
        t.rotate_y(std::f32::consts::PI);
    }
}

fn update_pickup_toast(
    time: Res<Time>,
    mut toast: ResMut<PickupToast>,
    mut q: Query<&mut Text, With<PickupToastText>>,
) {
    if toast.left_s > 0.0 {
        toast.left_s = (toast.left_s - time.delta_seconds().min(0.05)).max(0.0);
    }
    let Ok(mut t) = q.get_single_mut() else { return };
    if toast.left_s <= 0.0 {
        if !t.sections[0].value.is_empty() {
            t.sections[0].value.clear();
        }
        return;
    }
    t.sections[0].value = toast.text.clone();
}

fn use_medkit(
    keys: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<PlayerStats>,
    mut cons: ResMut<PlayerConsumables>,
    mut toast: ResMut<PickupToast>,
) {
    if !keys.just_pressed(KeyCode::KeyH) {
        return;
    }
    if cons.medkits <= 0 {
        toast.text = "血包库存为 0".to_string();
        toast.left_s = 1.2;
        return;
    }
    if stats.hp >= 100 {
        toast.text = "HP 已满".to_string();
        toast.left_s = 1.2;
        return;
    }
    cons.medkits -= 1;
    let heal = 25;
    stats.hp = (stats.hp + heal).min(100);
    toast.text = format!("使用血包：+{}（HP {}，库存 {}）", heal, stats.hp, cons.medkits);
    toast.left_s = 1.4;
}

fn setup_boss_arena(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    spawn_world: Vec3,
) {
    // 大空间：地面 + 外墙 + 柱子掩体 + Boss
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.11),
        perceptual_roughness: 0.95,
        ..default()
    });
    let wall_tex: Handle<Image> = assets.load("textures/wall_cc0.png");
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        base_color: Color::srgb(0.85, 0.85, 0.88),
        emissive: LinearRgba::new(0.03, 0.03, 0.035, 1.0),
        perceptual_roughness: 0.98,
        ..default()
    });

    let arena = 90.0;
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(arena, 0.2, arena)),
            material: floor_mat,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Collider::cuboid(arena * 0.5, 0.1, arena * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));
    // 墙
    let wall_h = 3.0;
    let thick = 0.35;
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(thick, wall_h, arena)),
            material: wall_mat.clone(),
            transform: Transform::from_xyz(-arena * 0.5, wall_h * 0.5, 0.0),
            ..default()
        },
        Collider::cuboid(thick * 0.5, wall_h * 0.5, arena * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(thick, wall_h, arena)),
            material: wall_mat.clone(),
            transform: Transform::from_xyz(arena * 0.5, wall_h * 0.5, 0.0),
            ..default()
        },
        Collider::cuboid(thick * 0.5, wall_h * 0.5, arena * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(arena, wall_h, thick)),
            material: wall_mat.clone(),
            transform: Transform::from_xyz(0.0, wall_h * 0.5, -arena * 0.5),
            ..default()
        },
        Collider::cuboid(arena * 0.5, wall_h * 0.5, thick * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(arena, wall_h, thick)),
            material: wall_mat,
            transform: Transform::from_xyz(0.0, wall_h * 0.5, arena * 0.5),
            ..default()
        },
        Collider::cuboid(arena * 0.5, wall_h * 0.5, thick * 0.5),
        RigidBody::Fixed,
        WorldEntity,
    ));

    // 掩体柱子（简单 3x3）
    let pillar_mesh = meshes.add(Cuboid::new(2.2, 3.0, 2.2));
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.25),
        perceptual_roughness: 0.98,
        ..default()
    });
    for z in [-20.0, 0.0, 20.0] {
        for x in [-20.0, 0.0, 20.0] {
            if x == 0.0 && z == 0.0 {
                continue;
            }
            commands.spawn((
                PbrBundle {
                    mesh: pillar_mesh.clone(),
                    material: pillar_mat.clone(),
                    transform: Transform::from_xyz(x, 1.5, z),
                    ..default()
                },
                Collider::cuboid(1.1, 1.5, 1.1),
                RigidBody::Fixed,
                WorldEntity,
            ));
        }
    }

    // Boss（用 enemy sprite 但更大）
    let enemy_tex: Handle<Image> = assets.load("textures/enemy_cc0.png");
    let boss_mat = materials.add(StandardMaterial {
        base_color_texture: Some(enemy_tex),
        base_color: Color::srgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let boss_mesh = meshes.add(make_billboard_mesh(3.4, 4.6, enemy_frame_uv(0)));
    // 离墙稍远，避免物理初始穿插导致“卡墙不动”
    let boss_pos = Vec3::new(0.0, 1.2, -24.0);
    commands.spawn((
        PbrBundle {
            mesh: boss_mesh,
            material: boss_mat,
            transform: Transform::from_translation(boss_pos),
            ..default()
        },
        Enemy,
        Boss,
        Health { hp: 300 },
        BossAi {
            fireball_cd_s: 1.8,
            volley_cd_s: 4.0,
            volley_left: 0,
            volley_interval_s: 0.6,
            volley_tick_s: 0.0,
            summon_cooldown_s: 0.0,
            summon_since_s: 0.0,
        },
        RigidBody::Dynamic,
        GravityScale(0.0),
        Ccd::enabled(),
        Damping {
            linear_damping: 4.0,
            angular_damping: 8.0,
        },
        Velocity::zero(),
        Collider::capsule_y(0.85, 0.65),
        LockedAxes::ROTATION_LOCKED,
        WorldEntity,
    ));

    // 玩家出生点稍微靠前
    let _ = spawn_world;
}

fn boss_ai(
    time: Res<Time>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    mut bosses: Query<(&GlobalTransform, &mut BossAi, &Health), With<Boss>>,
    minions: Query<Entity, With<Minion>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    let Ok(p) = player.get_single() else { return };
    let ppos = p.translation();
    let dt = time.delta_seconds().min(0.05);

    for (boss_t, mut ai, hp) in &mut bosses {
        if hp.hp <= 0 {
            continue;
        }
        ai.fireball_cd_s = (ai.fireball_cd_s - dt).max(0.0);
        ai.volley_cd_s = (ai.volley_cd_s - dt).max(0.0);
        ai.summon_cooldown_s = (ai.summon_cooldown_s - dt).max(0.0);
        ai.summon_since_s += dt;

        let origin = boss_t.translation() + Vec3::new(0.0, 1.2, 0.0);

        // 追踪弹齐射：cd 到了就启动 5 发 / 3 秒
        if ai.volley_left <= 0 && ai.volley_cd_s <= 0.0 {
            ai.volley_left = 5;
            ai.volley_tick_s = 0.0;
            ai.volley_cd_s = 7.5;
        }
        if ai.volley_left > 0 {
            ai.volley_tick_s += dt;
            while ai.volley_left > 0 && ai.volley_tick_s >= ai.volley_interval_s {
                ai.volley_tick_s -= ai.volley_interval_s;
                ai.volley_left -= 1;
                let dir = (ppos - origin).normalize_or_zero();
                spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    origin,
                    dir,
                    18.0,
                    6,
                    6.0,
                    Color::srgb(0.9, 0.7, 0.2),
                );
            }
        }

        // 慢火球：高伤害，可躲
        if ai.fireball_cd_s <= 0.0 {
            ai.fireball_cd_s = 2.6;
            let dir = (ppos - origin).normalize_or_zero();
            spawn_projectile(
                &mut commands,
                &mut meshes,
                &mut materials,
                origin,
                dir,
                8.0,
                18,
                10.0,
                Color::srgb(1.0, 0.25, 0.15),
            );
        }

        // 召唤：场上无小兵且 >30s，低概率触发
        if minions.is_empty() && ai.summon_since_s > 30.0 && ai.summon_cooldown_s <= 0.0 {
            let roll = rand::random::<f32>();
            if roll < 0.02 {
                ai.summon_cooldown_s = 10.0;
                ai.summon_since_s = 0.0;
                let count = 2 + (rand::random::<u32>() % 3) as i32;
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                    let pos = origin + Vec3::new(angle.cos() * 6.0, -0.9, angle.sin() * 6.0);
                    spawn_minion(&mut commands, &mut meshes, &mut materials, &assets, pos);
                }
            }
        }
    }
}

fn spawn_projectile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    dir: Vec3,
    speed: f32,
    damage: i32,
    life_s: f32,
    color: Color,
) {
    let d = dir.normalize_or_zero();
    if d == Vec3::ZERO {
        return;
    }
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(0.28)),
            material: materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * 0.8,
                ..default()
            }),
            transform: Transform::from_translation(origin),
            ..default()
        },
        Projectile {
            vel: d * speed,
            damage,
            left_s: life_s,
        },
        WorldEntity,
    ));
}

fn spawn_minion(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    pos: Vec3,
) {
    let enemy_tex: Handle<Image> = assets.load("textures/enemy_cc0.png");
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(enemy_tex),
        base_color: Color::srgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let mesh = meshes.add(make_billboard_mesh(1.9, 2.6, enemy_frame_uv(0)));
    commands.spawn((
        PbrBundle {
            mesh,
            material: mat,
            transform: Transform::from_translation(pos),
            ..default()
        },
        Enemy,
        Minion,
        Health { hp: 30 },
        EnemyAi {
            speed: 1.2,
            attack_range: 1.15,
            attack_cooldown_s: 1.6,
            damage: 3,
            cooldown_left_s: 0.0,
        },
        EnemyAnim {
            walk_seq: ENEMY_WALK_SEQ.to_vec(),
            hit_seq: ENEMY_HIT_SEQ.to_vec(),
            die_seq: ENEMY_DIE_SEQ.to_vec(),
            state: EnemyAnimState::Walk,
            frame_i: 0,
            hit_left_s: 0.0,
            death_hold_left_s: 0.0,
            fps: 10.0,
            acc: 0.0,
        },
        RigidBody::Dynamic,
        GravityScale(0.0),
        Ccd::enabled(),
        Damping {
            linear_damping: 4.0,
            angular_damping: 8.0,
        },
        Velocity::zero(),
        Collider::capsule_y(0.55, 0.42),
        LockedAxes::ROTATION_LOCKED,
        WorldEntity,
    ));
}

fn projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut stats: ResMut<PlayerStats>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    mut q: Query<(Entity, &mut Transform, &mut Projectile)>,
    mut hermes: EventWriter<HermesEvent>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Ok(p) = player.get_single() else { return };
    let ppos = p.translation();
    let dt = time.delta_seconds().min(0.05);
    for (e, mut t, mut pr) in &mut q {
        pr.left_s -= dt;
        if pr.left_s <= 0.0 {
            commands.entity(e).despawn_recursive();
            continue;
        }
        t.translation += pr.vel * dt;
        if t.translation.distance(ppos) < 0.75 {
            stats.hp -= pr.damage;
            commands.entity(e).despawn_recursive();
            hermes.send(HermesEvent {
                topic: HermesTopic::Gameplay,
                from: AgentRole::GameplayProgrammer,
                message: format!("玩家受伤：-{}（HP {}）", pr.damage, stats.hp),
            });
            if stats.hp <= 0 {
                stats.hp = 100;
                hermes.send(HermesEvent {
                    topic: HermesTopic::ProducerGate,
                    from: AgentRole::Producer,
                    message: "玩家死亡，返回主菜单".to_string(),
                });
                next.set(AppState::MainMenu);
                return;
            }
        }
    }
}

fn boss_victory_check(
    floor: Res<FloorState>,
    boss: Query<&Health, With<Boss>>,
    mut next: ResMut<NextState<AppState>>,
    mut hermes: EventWriter<HermesEvent>,
) {
    if floor.floor != 5 {
        return;
    }
    let Ok(hp) = boss.get_single() else { return };
    if hp.hp > 0 {
        return;
    }
    hermes.send(HermesEvent {
        topic: HermesTopic::ProducerGate,
        from: AgentRole::Producer,
        message: "Boss 已击败：通关 5 层，返回主菜单".to_string(),
    });
    next.set(AppState::MainMenu);
}

fn pickup_key(
    mut commands: Commands,
    mut inventory: ResMut<Inventory>,
    audio: Res<AudioAssets>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    keys_q: Query<(Entity, &Transform), With<Key>>,
    mut hermes: EventWriter<HermesEvent>,
) {
    if inventory.has_key {
        return;
    }
    let Ok(p) = player.get_single() else { return };

    for (e, t) in &keys_q {
        if p.translation().distance(t.translation) < 1.4 {
            inventory.has_key = true;
            commands.entity(e).despawn_recursive();
            commands.spawn(AudioBundle {
                source: audio.pickup.clone(),
                settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.8)),
            });
            hermes.send(HermesEvent {
                topic: HermesTopic::Gameplay,
                from: AgentRole::GameplayProgrammer,
                message: "玩家拾取钥匙".to_string(),
            });
            break;
        }
    }
}

fn try_open_door(
    keys: Res<ButtonInput<KeyCode>>,
    inventory: Res<Inventory>,
    audio: Res<AudioAssets>,
    player: Query<&GlobalTransform, With<PlayerBody>>,
    mut doors: Query<(&Transform, &mut Door, Entity)>,
    mut hermes: EventWriter<HermesEvent>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok(p) = player.get_single() else { return };

    for (t, mut door, e) in &mut doors {
        if p.translation().distance(t.translation) > 2.6 {
            continue;
        }

        if door.locked && !inventory.has_key {
            hermes.send(HermesEvent {
                topic: HermesTopic::ProducerGate,
                from: AgentRole::Producer,
                message: "门是锁着的（需要钥匙）".to_string(),
            });
            continue;
        }

        door.locked = false;
        commands.entity(e).despawn_recursive();
        commands.spawn(AudioBundle {
            source: audio.door_open.clone(),
            settings: PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(0.8)),
        });
        hermes.send(HermesEvent {
            topic: HermesTopic::Gameplay,
            from: AgentRole::GameplayProgrammer,
            message: "门已打开".to_string(),
        });
    }
}

fn hermes_debug_log(mut hermes: EventReader<HermesEvent>) {
    for ev in hermes.read() {
        info!("[hermes][{:?}][{:?}] {}", ev.topic, ev.from, ev.message);
    }
}

