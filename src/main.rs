use bevy::{
    input::{keyboard::KeyCode, Input},
    prelude::*,
    sprite::MaterialMesh2dBundle,
    time::FixedTimestep,
};
use std::f32::consts::PI;
use rand::Rng;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK)) // Sets background color to black
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
            .with_run_criteria(FixedTimestep::step(1. / 60.))
            .with_system(move_paddle)
            .with_system(move_ball)
            .with_system(collision.after(move_ball).after(move_paddle))
            .with_system(score.after(move_ball))
        )
        .add_event::<BallResetEvent>()
        .add_system(reset_ball)
        .add_system(check_for_win)
        .run();
}

#[derive(Component)]
struct Score (u16);

#[derive(Component, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

// Mesured in radians counterclockwise starting from positive x axis
#[derive(Component)]
struct BallDirection (f32);

#[derive(Component)]
struct BallStartingPoint {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct DistanceFromStartingPoint (f32);

#[derive(Component)]
enum Collision {
    Left,
    Right,
    Top,
    Bottom,
}

const TABLE_SIZE: [f32; 2] = [1400., 700.]; // [x, y]

const PADDLE_LENGTH: f32 = 100.;
const PADDLE_WIDTH: f32 = 20.;
const PADDLE_SPEED: f32 = 5.0;

const BALL_RADIUS: f32 = 15.;
const BALL_SPEED: f32 = 6.;

struct BallResetEvent;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {

    commands.spawn_bundle(Camera2dBundle::default());

    // Generating big white cube for border
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(TABLE_SIZE[0] + 50., TABLE_SIZE[1] + 50.)),
            ..default()
        },
        ..default()
    });

    // Black cube for border also the game zone
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::new(TABLE_SIZE[0], TABLE_SIZE[1])),
            ..default()
        },
        ..default()
        });

    // Table middle line
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(20., TABLE_SIZE[1])),
            ..default()
        },
        ..default()
    });

    // Add black stripes to middle line
    for i in -20..20 as i16 {
        if i % 2 == 0 {
            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::BLACK,
                    custom_size: Some(Vec2::new(20., 50.)),
                    ..default()
                },
                transform: Transform::from_xyz(0., f32::from(i * 50), 2.),
                ..default()
            });
        }
    }

    // Score number style
    let score_style = TextStyle {
        font: asset_server.load("Lato-Bold.ttf"),
        font_size: 120.,
        color: Color::WHITE,
    };

    // Left score counter
    commands.spawn_bundle(Text2dBundle {
        text: Text::from_section("0", score_style.clone())
            .with_alignment(TextAlignment::CENTER),
        transform: Transform::from_xyz(-60., TABLE_SIZE[1] / 2. - 50., 2.),
        ..default()
    })
    .insert(Side::Left);

    // Right score counter
    commands.spawn_bundle(Text2dBundle {
        text: Text::from_section("0", score_style)
            .with_alignment(TextAlignment::CENTER),
        transform: Transform::from_xyz(60., TABLE_SIZE[1] / 2. - 50., 2.),
        ..default()
    })
    .insert(Side::Right);

    // Left player
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(PADDLE_WIDTH, PADDLE_LENGTH)),
            ..default()
        },
        transform: Transform::from_xyz(-600., 0., 2.),
        ..default()
    })
    .insert(Score(0))
    .insert(Side::Left);

    // Right player
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(PADDLE_WIDTH, PADDLE_LENGTH)),
            ..default()
        },
        transform: Transform::from_xyz(600., 0., 2.),
        ..default()
    })
    .insert(Score (0))
    .insert(Side::Right);

    // The ball
    commands.spawn_bundle(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(BALL_RADIUS).into()).into(),
        material: materials.add(ColorMaterial::from(Color::WHITE)),
        transform: Transform::from_xyz(0., 0., 3.),
        ..default()
    })
    .insert(BallDirection (ball_first_direction()))
    .insert(BallStartingPoint { x: 0., y: 0. })
    .insert(DistanceFromStartingPoint (0.));
}

fn move_paddle(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &Side), With<Score>>,
) {

    for (mut transform, side) in &mut query {

        // Moves paddle
        match side {
            Side::Left => {
                // Move left paddle with W and S
                if keyboard_input.pressed(KeyCode::W) {
                    transform.translation.y += PADDLE_SPEED;
                }
                if keyboard_input.pressed(KeyCode::S) {
                    transform.translation.y -= PADDLE_SPEED;
                }
            }
            Side::Right => {
                // Move right paddle with Up and Down arrow keys
                if keyboard_input.pressed(KeyCode::Up) {
                    transform.translation.y += PADDLE_SPEED;
                }
                if keyboard_input.pressed(KeyCode::Down) {
                    transform.translation.y -= PADDLE_SPEED;
                }
            }
        }

        // Doesn't let paddle exit game zone
        if transform.translation.y + PADDLE_LENGTH / 2. >= TABLE_SIZE[1] / 2. {
            transform.translation.y -= PADDLE_SPEED;
        } else if transform.translation.y - PADDLE_LENGTH / 2. <= -TABLE_SIZE[1] / 2. {
            transform.translation.y += PADDLE_SPEED;
        }
    }
}

fn move_ball(
    mut query: Query<(
        &mut Transform,
        &BallDirection,
        &BallStartingPoint,
        &mut DistanceFromStartingPoint,
    )>) {

    for (
        mut transform,
        direction,
        starting_point,
        mut distance
        ) in &mut query {

        // Distance between ball and ball starting point
        distance.0 += BALL_SPEED;

        // Sarting point x coordinate
        let x1 = starting_point.x;
        // Sarting point y coordinate
        let y1 = starting_point.y;

        // Ball x coordinate
        let x = x1 + direction.0.cos() * distance.0;
        // Ball y coordinate
        let y = y1 + direction.0.sin() * distance.0;

        // Change the transform
        transform.translation.x = x;
        transform.translation.y = y;
    }
}

fn collision(
    mut ball_query: Query<(
        &Transform,
        &mut BallDirection,
        &mut BallStartingPoint,
        &mut DistanceFromStartingPoint
    )>,
    paddle_query: Query<(&Transform, &Side), With<Score>>,
) {

    for (transform,
        mut direction,
        mut starting_point,
        mut distance,
        ) in &mut ball_query {

        // Check for collision with play area
        let mut collision = match transform.translation.y {
            y if y + BALL_RADIUS >= TABLE_SIZE[1] / 2. => Some(Collision::Top),
            y if y - BALL_RADIUS <= -TABLE_SIZE[1] / 2. => Some(Collision::Bottom),
            _ => None
        };

        // If no play area collision found check for collision with paddle
        if collision.is_none() {
            for (paddle_transform, paddle_side) in &paddle_query {

                // Check if ball is past paddles on x axis
                match paddle_side {
                    Side::Right => {
                        if transform.translation.x + BALL_RADIUS >= paddle_transform.translation.x + PADDLE_WIDTH / 2. {break}
                    },
                    Side::Left => {
                        if transform.translation.x - BALL_RADIUS <= paddle_transform.translation.x - PADDLE_WIDTH / 2. {break}
                    }
                }

                // Left and right collisions with paddle
                // Check y axis
                if transform.translation.y - BALL_RADIUS <= paddle_transform.translation.y + PADDLE_LENGTH / 2. && 
                transform.translation.y + BALL_RADIUS >= paddle_transform.translation.y - PADDLE_LENGTH / 2. {

                    // Check x axis
                    match paddle_side {
                        Side::Right => {
                            if transform.translation.x + BALL_RADIUS >= paddle_transform.translation.x - PADDLE_WIDTH / 2. {
                                collision = Some(Collision::Right);
                            }
                        }
                        Side::Left => {
                            if transform.translation.x - BALL_RADIUS <= paddle_transform.translation.x + PADDLE_WIDTH / 2. {
                                collision = Some(Collision::Left);
                            }
                        }
                    }
                }

                // Top and bottom collisions with paddle
                // Check x axis
                if transform.translation.x + BALL_RADIUS >= paddle_transform.translation.x - PADDLE_WIDTH / 2. &&
                transform.translation.x - BALL_RADIUS <= paddle_transform.translation.x + PADDLE_WIDTH / 2. {

                    // Check y axis
                    collision = match transform.translation.y {
                        y if y - BALL_RADIUS <= paddle_transform.translation.y + PADDLE_LENGTH / 2. &&
                        y - BALL_RADIUS >= paddle_transform.translation.y + PADDLE_LENGTH / 2. - (BALL_SPEED + PADDLE_SPEED) / 2. => Some(Collision::Top),

                        y if y + BALL_RADIUS >= paddle_transform.translation.y - PADDLE_LENGTH / 2. &&
                        y - BALL_RADIUS <= paddle_transform.translation.y - PADDLE_LENGTH / 2. + (BALL_SPEED + PADDLE_SPEED) / 2. => Some(Collision::Bottom),

                        _ => collision
                    };
                }
            }
        }

        // If collision found
        if let Some(collision) = collision {
            // Change ball direction
            direction.0 = match collision {
                Collision::Top => 2. * PI - direction.0,
                Collision::Bottom => 2. * PI - direction.0,
                Collision::Right => PI - direction.0,
                Collision::Left => PI - direction.0,
            };

            // Change starting point
            starting_point.x = transform.translation.x;
            starting_point.y = transform.translation.y;

            // Set distance to 0
            distance.0 = 0.;
        }
    }
}

fn score(
    ball_query: Query<&Transform, With<BallDirection>>,
    mut player_query: Query<(&mut Score, &Side)>,
    mut score_counter_query: Query<(&mut Text, &Side)>,
    mut events: EventWriter<BallResetEvent>,
) {

    for ball_transform in &ball_query {
        // Check if somebody received a point
        let point_side = match ball_transform.translation.x {
            x if x + BALL_RADIUS / 2. >= TABLE_SIZE[0] / 2. => Some(Side::Right),
            x if x - BALL_RADIUS / 2. <= 0. - TABLE_SIZE[0] / 2. => Some(Side::Left),
            _ => None,
        };

        if let Some(point_side) = point_side {
            for (mut score, player_side) in &mut player_query {

                // Increase player score if he made the point
                if player_side == &point_side {
                    score.0 += 1;

                    // Change score counter
                    for (mut text, counter_side) in &mut score_counter_query {
                        if counter_side == &point_side {
                            text.sections[0].value = score.0.to_string();
                        }
                    }

                    events.send(BallResetEvent);
                }
            }
        }
    }
}

fn reset_ball(
    mut ball_query: Query<(&mut Transform, &mut BallStartingPoint, &mut DistanceFromStartingPoint, &mut BallDirection)>,
    mut event_reader: EventReader<BallResetEvent>,
) {

    for _ in event_reader.iter() {
        for (mut transform,
            mut starting_point,
            mut distance,
            mut direction
            ) in &mut ball_query {

            transform.translation.x = 0.;
            transform.translation.y = 0.;
            starting_point.x = 0.;
            starting_point.y = 0.;
            distance.0 = 0.;
            direction.0 = ball_first_direction();
        }
    }
}

fn ball_first_direction() -> f32 {
    let mut rng = rand::thread_rng();

    let side = match rng.gen_bool(0.5) {
        true => Side::Left,
        false => Side::Right,
    };

    match side {
        Side::Left => {
            let mut direction = rng.gen_range((-PI / 3.)..(PI / 3.));
            while direction >= -0.1 && direction <= 0.1 {
                direction = rng.gen_range((-PI / 3.)..(PI / 3.));
            }
            direction
        }
        Side::Right => {
            let mut direction = rng.gen_range((2. * PI / 3.)..(4. * PI / 3.));
            while direction >= PI - 0.1 && direction <= PI + 0.1 {
                direction = rng.gen_range((-PI / 3.)..(PI / 3.));
            }
            direction
        }
    }
}

fn check_for_win(score_query: Query<&mut Score>, mut app_exit_events: ResMut<Events<bevy::app::AppExit>>) {
    for score in &score_query {
        if score.0 == 10 {
            app_exit_events.send(bevy::app::AppExit);
        }
    }
}