use std::collections::HashMap;
use std::iter;

use piet::kurbo::{Point, Rect};
use piet::{RenderContext, Text, TextLayout, TextLayoutBuilder};
use piet_cairo::{CairoRenderContext, CairoText};
use resvg;
use usvg;

use crate::weather::{
    AtmosphereType, CloudsType, OpenWeatherResponse, RainType, SnowType, WeatherCondition,
    WeatherState,
};

pub fn render(
    weather_report: Option<OpenWeatherResponse>,
    radar_map: Option<bytes::Bytes>,
    ctx: &mut CairoRenderContext,
) {
    if let Some(weather_report) = weather_report {
        draw_current_conditions(
            ctx,
            &weather_report.current,
            Rect::from_origin_size((0., 250.), (280., 120.)),
        );

        for (i, forecast) in weather_report.hourly.iter().take(5).enumerate() {
            draw_forecast(
                ctx,
                forecast,
                Rect::from_origin_size(((i * 56) as f64, 380.), (56., 100.)),
            );
        }
    }

    if let Some(radar_map) = radar_map {
        draw_weather_radar(
            ctx,
            radar_map,
            Rect::from_origin_size(Point::ORIGIN, (280., 240.)),
        );
    }
}

fn draw_current_conditions(ctx: &mut CairoRenderContext, state: &WeatherState, position: Rect) {
    ctx.with_save(|ctx| {
        ctx.clip(position);
        ctx.clear(piet::Color::from_rgba32_u32(0xFDFDFD));

        let icon_size = position.height() - 20.;
        let text_area_width = position.width() - icon_size;

        {
            let text = CairoText::new()
                .new_text_layout(format!("{}", state.temp))
                .default_attribute(piet::TextAttribute::FontSize(position.height() / 2.))
                .build()
                .unwrap();
            ctx.draw_text(
                &text,
                ((text_area_width - text.size().width) / 2., position.y0),
            );
        }

        {
            let wind_speed = CairoText::new()
                .new_text_layout(format!("{} km/h", state.wind.km_h()))
                .default_attribute(piet::TextAttribute::FontSize(position.height() / 6.))
                .build()
                .unwrap();
            let wind_direction = CairoText::new()
                .new_text_layout(state.wind.arrow())
                .default_attribute(piet::TextAttribute::FontSize(position.height() / 4.))
                .build()
                .unwrap();

            ctx.draw_text(
                &wind_speed,
                (
                    (text_area_width / 2. - wind_speed.size().width) / 2.,
                    position.y1 - wind_speed.size().height,
                ),
            );
            ctx.draw_text(
                &wind_direction,
                (
                    (text_area_width / 2. - wind_speed.size().width) / 2.,
                    position.y1 - wind_speed.size().height - wind_direction.size().height,
                ),
            );
        }

        {
            let icon = ctx
                .make_image(
                    icon_size as usize,
                    icon_size as usize,
                    &resvg::render(
                        &get_weather_icon(state),
                        usvg::FitTo::Height(icon_size as u32),
                        None,
                    )
                    .unwrap()
                    .data()[..],
                    piet::ImageFormat::RgbaPremul,
                )
                .unwrap();
            ctx.draw_image(
                &icon,
                Rect::from_origin_size(
                    (
                        position.x1 - icon_size - 10.,
                        position.y0 + (position.height() - icon_size) / 2.,
                    ),
                    (icon_size, icon_size),
                ),
                piet::InterpolationMode::NearestNeighbor,
            );
        }

        Ok(())
    })
    .unwrap()
}

fn draw_forecast(ctx: &mut CairoRenderContext, state: &WeatherState, position: Rect) {
    ctx.with_save(|ctx| {
        ctx.clip(position);
        ctx.clear(piet::Color::from_rgba32_u32(0xFDFDFD));

        let icon_size = position.height() / 2. - 10.;

        {
            let text = CairoText::new()
                .new_text_layout(format!("{}", state.temp))
                .default_attribute(piet::TextAttribute::FontSize(position.width() / 3.))
                .build()
                .unwrap();
            ctx.draw_text(
                &text,
                (
                    position.x0 + (position.width() - text.size().width) / 2.,
                    position.y0,
                ),
            );
        }

        {
            let icon = ctx
                .make_image(
                    icon_size as usize,
                    icon_size as usize,
                    &resvg::render(
                        &get_weather_icon(state),
                        usvg::FitTo::Height(icon_size as u32),
                        None,
                    )
                    .unwrap()
                    .data()[..],
                    piet::ImageFormat::RgbaPremul,
                )
                .unwrap();
            ctx.draw_image(
                &icon,
                Rect::from_origin_size(
                    (
                        position.x0 + (position.width() - icon_size) / 2.,
                        position.y0 + (position.height() - icon_size) / 2.,
                    ),
                    (icon_size, icon_size),
                ),
                piet::InterpolationMode::NearestNeighbor,
            );
        }

        {
            let text = CairoText::new()
                .new_text_layout(format!("{}h", state.time.hour()))
                .default_attribute(piet::TextAttribute::FontSize(position.width() / 3.))
                .build()
                .unwrap();
            ctx.draw_text(
                &text,
                (
                    position.x0 + (position.width() - text.size().width) / 2.,
                    position.y1 - text.size().height,
                ),
            );
        }

        Ok(())
    })
    .unwrap();
}

fn draw_weather_radar(ctx: &mut CairoRenderContext, radar_map: bytes::Bytes, position: Rect) {
    ctx.with_save(|ctx| {
        ctx.clip(position);

        {
            let mut decoder =
                gif::Decoder::new(&include_bytes!("../images/radar-rivers.gif")[..]).unwrap();
            let (palette, frame, frame_width, frame_height) = {
                let palette: Vec<u8> = decoder.palette().unwrap().iter().copied().collect();
                let frame = decoder.read_next_frame().unwrap().unwrap();

                (palette, frame, frame.width as usize, frame.height as usize)
            };

            let rivers = ctx
                .make_image(
                    frame_width,
                    frame_height,
                    &frame
                        .buffer
                        .iter()
                        .flat_map(|color: &u8| {
                            iter::repeat(0x55).take(3).chain(iter::once(
                                0xFF - palette.get((color * 3) as usize).unwrap_or(&0x00),
                            ))
                        })
                        .collect::<Vec<u8>>()[..],
                    piet::ImageFormat::RgbaPremul,
                )
                .unwrap();

            ctx.draw_image_area(
                &rivers,
                Rect::from_center_size(
                    (frame_height as f64 / 2., frame_height as f64 / 2.),
                    position.size(),
                ),
                position,
                piet::InterpolationMode::Bilinear,
            );
        }

        {
            let mut decoder = gif::Decoder::new(&radar_map[..]).unwrap();
            let (palette, frame, frame_width, frame_height) = {
                let frame = decoder.read_next_frame().unwrap().unwrap();

                let mut scale: Vec<u8> = Vec::new();

                frame
                    .buffer
                    .iter()
                    .skip(524)
                    .step_by(frame.width as usize)
                    .for_each(|pixel| {
                        if !scale.contains(pixel) {
                            scale.push(*pixel);
                        }
                    });

                let mut palette: HashMap<u8, u8> = HashMap::new();
                for i in (0x55..0xFF).step_by(0x2a) {
                    if let Some(index) = scale.pop() {
                        palette.insert(index, i);
                    }
                }
                while let Some(index) = scale.pop() {
                    palette.insert(index, 0xFF);
                }

                (palette, frame, frame.width as usize, frame.height as usize)
            };

            let radar_map = ctx
                .make_image(
                    frame_width,
                    frame_height,
                    &frame
                        .buffer
                        .iter()
                        .flat_map(|color: &u8| {
                            iter::repeat(0x00)
                                .take(3)
                                .chain(iter::once(palette.get(color).unwrap_or(&0x00)).copied())
                        })
                        .collect::<Vec<u8>>()[..],
                    piet::ImageFormat::RgbaPremul,
                )
                .unwrap();
            ctx.draw_image_area(
                &radar_map,
                Rect::from_center_size(
                    (frame_height as f64 / 2., frame_height as f64 / 2.),
                    position.size(),
                ),
                position,
                piet::InterpolationMode::Bilinear,
            );
        }

        Ok(())
    })
    .unwrap();
}

fn get_weather_icon(state: &WeatherState) -> usvg::Tree {
    let daytime = state.time > state.sunrise && state.time < state.sunset;
    let partly_cloudy = state.clouds <= 50;

    usvg::Tree::from_str(
        match &state.condition {
            WeatherCondition::Thunderstorm(_) if partly_cloudy => {
                if daytime {
                    include_str!("../images/weather/022-storm-1.svg")
                } else {
                    include_str!("../images/weather/042-storm-2.svg")
                }
            }
            WeatherCondition::Thunderstorm(_) => include_str!("../images/weather/043-rain-1.svg"),
            WeatherCondition::Drizzle(_) => include_str!("../images/weather/046-drizzle.svg"),
            WeatherCondition::Rain(subtype) => match subtype {
                RainType::FreezingRain => include_str!("../images/weather/014-icicles.svg"),
                _ if partly_cloudy => {
                    if daytime {
                        include_str!("../images/weather/011-cloudy.svg")
                    } else {
                        include_str!("../images/weather/013-night-1.svg")
                    }
                }
                _ => {
                    include_str!("../images/weather/004-rainy.svg")
                }
            },
            WeatherCondition::Snow(subtype) => match subtype {
                SnowType::Sleet | SnowType::LightShowerSleet | SnowType::ShowerSleet => {
                    include_str!("../images/weather/031-hail.svg")
                }
                SnowType::LightRainAndSnow
                | SnowType::RainAndSnow
                | SnowType::LightShowerSnow
                | SnowType::ShowerSnow
                | SnowType::HeavyShowerSnow => include_str!("../images/weather/024-snowy.svg"),
                _ if partly_cloudy => {
                    if daytime {
                        include_str!("../images/weather/032-snowy-1.svg")
                    } else {
                        include_str!("../images/weather/002-night.svg")
                    }
                }
                _ => include_str!("../images/weather/041-snowy-2.svg"),
            },
            WeatherCondition::Atmosphere(subtype) => match subtype {
                AtmosphereType::Tornado => include_str!("../images/weather/016-tornado-1.svg"),
                AtmosphereType::Squalls => include_str!("../images/weather/015-windy-1.svg"),
                _ => include_str!("../images/weather/045-fog.svg"),
            },
            WeatherCondition::Clear if daytime => include_str!("../images/weather/044-sun.svg"),
            WeatherCondition::Clear => include_str!("../images/weather/034-night-3.svg"),
            WeatherCondition::Clouds(subtype) => match subtype {
                CloudsType::FewClouds | CloudsType::ScatteredClouds if daytime => {
                    include_str!("../images/weather/033-cloudy-3.svg")
                }
                CloudsType::FewClouds => {
                    include_str!("../images/weather/023-night-2.svg")
                }
                CloudsType::ScatteredClouds => {
                    include_str!("../images/weather/028-cloudy-2.svg")
                }
                _ => include_str!("../images/weather/035-cloudy-4.svg"),
            },
            WeatherCondition::Unknown(_) => include_str!("../images/weather/019-weathercock.svg"),
        },
        &usvg::Options::default(),
    )
    .unwrap()
}
