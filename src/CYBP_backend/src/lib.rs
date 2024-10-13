use std::time::{SystemTime, UNIX_EPOCH};
use ic_cdk_macros::{query, update};
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs, TransformContext,
};
use serde::{Serialize, Deserialize};
use candid::{CandidType};
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};

#[derive(CandidType)]
struct Response {
    timestamp: i64,
}

// Define a struct to hold the price data for regression
#[derive(CandidType, Serialize, Deserialize)]
pub struct RegressionInput {
    x: Vec<f64>,
    y: Vec<f64>,
}

// Define a struct for the regression output
#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct RegressionOutput {
    parameters: Vec<(String, f64)>,
    intercept: f64,
}

// Struct to parse the API response
#[derive(Debug, Deserialize)]
struct ApiResponse {
    prices: Vec<Vec<f64>>,
}

#[update]
async fn get_icp_usd_prices(date_str: Option<String>) -> Result<(RegressionOutput, Option<f64>), String> {
    let url = "https://api.coingecko.com/api/v3/coins/internet-computer/market_chart?vs_currency=usd&days=365";
    let request_headers = vec![
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "price_fetcher_canister".to_string(),
        },
    ];

    let request = CanisterHttpRequestArgument {
        url: url.to_string(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: None,
        transform: Some(TransformContext::from_name("transform".to_string(), vec![])),
        headers: request_headers,
    };

    match http_request(request, 1_603_114_000).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body).expect("Response is not UTF-8 encoded.");
            let api_response: ApiResponse = serde_json::from_str(&str_body).expect("Failed to parse JSON");

            ic_cdk::api::print(format!("API Response: {:?}", api_response));

            let x: Vec<f64> = api_response.prices.iter().map(|p| p[0] / 1_000.0).collect(); // Convert to seconds
            let y: Vec<f64> = api_response.prices.iter().map(|p| p[1]).collect();

            // Create RegressionInput from extracted data
            let regression_input = RegressionInput { x, y };

            // Prepare the data for regression
            let data = vec![
                ("Y", regression_input.y),
                ("X", regression_input.x),
            ];
            let regression_data = RegressionDataBuilder::new()
                .build_from(data)
                .map_err(|e| e.to_string())?;

            // Define the formula for the regression
            let formula = "Y ~ X";

            // Build and fit the model
            let model = FormulaRegressionBuilder::new()
                .data(&regression_data)
                .formula(formula)
                .fit()
                .map_err(|e| e.to_string())?;

            // Access parameters
            let parameters = model.parameters(); // This is usually a Vec<f64>

            // Assuming intercept is the first parameter
            let intercept = parameters[0];
            let coefficients = &parameters[1..]; // Coefficients for X

            // Prepare the output
            let result_parameters: Vec<(String, f64)> = coefficients.iter()
                .enumerate()
                .map(|(i, &value)| (format!("X{}", i + 1), value)) // Assuming coefficients are for X
                .collect();

            let output = RegressionOutput {
                parameters: result_parameters,
                intercept,
            };

            // If a date string is provided, calculate and return the predicted price
            let predicted_price = date_str.map(|date| {
                let response = date_to_unix_timestamp(date);
                let timestamp = response.timestamp as f64;
                predict_price(timestamp, &output)
            });

            Ok((output, predicted_price))
        }
        Err((r, m)) => {
            ic_cdk::api::print(format!("HTTP request error. Code: {:?}, Message: {}", r, m));
            Err("Failed to fetch prices".to_string())
        }
    }
}

fn predict_price(timestamp: f64, regression_output: &RegressionOutput) -> f64 {
    let intercept = regression_output.intercept;

    intercept + regression_output.parameters.iter()
        .find(|(name, _)| name == "X1")
        .map_or(0.0, |(_, coef)| coef * timestamp)
}

#[query]
fn transform(raw: TransformArgs) -> HttpResponse {
    let headers = vec![
        HttpHeader {
            name: "Content-Security-Policy".to_string(),
            value: "default-src 'self'".to_string(),
        },
        // Other headers...
    ];

    HttpResponse {
        status: raw.response.status.clone(),
        body: raw.response.body,
        headers,
        ..Default::default()
    }
}

// Function to calculate the Unix timestamp
fn calculate_unix_timestamp(year: i32, month: u32, day: u32) -> i64 {
    let mut total_days = 0;
    
    // Calculate days for years
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Calculate days for months in the current year
    for m in 1..month {
        total_days += days_in_month(m, year);
    }

    // Add the days in the current month
    total_days += day as i64 - 1; // Subtract 1 because we start counting from 0

    // Each day has 86400 seconds
    total_days * 86400
}

// Function to determine if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// Function to return the number of days in a given month
fn days_in_month(month: u32, year: i32) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0, // Invalid month
    }
}

#[ic_cdk::update]
fn date_to_unix_timestamp(date_str: String) -> Response {
    // Parse the year, month, and day from the input string
    let year: i32 = date_str[0..4].parse().unwrap();
    let month: u32 = date_str[4..6].parse().unwrap();
    let day: u32 = date_str[6..8].parse().unwrap();

    // Calculate the Unix timestamp
    let timestamp = calculate_unix_timestamp(year, month, day);

    Response { timestamp }
}

ic_cdk::export_candid!();