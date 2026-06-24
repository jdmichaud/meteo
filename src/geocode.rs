use reqwest::blocking::Client;
use serde::Deserialize;

pub struct Location {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct GeoResponse {
    features: Vec<GeoFeature>,
}

#[derive(Deserialize)]
struct GeoFeature {
    geometry: GeoGeometry,
    properties: GeoProperties,
}

#[derive(Deserialize)]
struct GeoGeometry {
    coordinates: [f64; 2], // GeoJSON order: [longitude, latitude]
}

#[derive(Deserialize)]
struct GeoProperties {
    city: Option<String>,
    label: String,
}

pub fn geocode(client: &Client, postcode: &str) -> Result<Location, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api-adresse.data.gouv.fr/search/?q={}&type=municipality&limit=1",
        postcode
    );

    let resp: GeoResponse = client.get(&url)
        .send()?
        .error_for_status()?
        .json()?;

    let feat = resp
        .features
        .into_iter()
        .next()
        .ok_or_else(|| format!("postcode '{}' not found", postcode))?;

    Ok(Location {
        name: feat.properties.city.unwrap_or(feat.properties.label),
        latitude:  feat.geometry.coordinates[1],
        longitude: feat.geometry.coordinates[0],
    })
}
