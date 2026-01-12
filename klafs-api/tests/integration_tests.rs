//! Integration tests using wiremock for mock HTTP server

use klafs_api::{ClientConfig, DebugConfig, KlafsClient, KlafsError, SaunaMode};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Load a fixture file
fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{}", name))
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", name))
}

/// Create a test client configured to use the mock server
fn create_test_client(mock_server: &MockServer) -> KlafsClient {
    let config = ClientConfig {
        base_url: mock_server.uri(),
        debug: DebugConfig::enabled(),
        timeout_secs: 5,
    };
    KlafsClient::with_config(config)
}

mod login_tests {
    use super::*;

    #[tokio::test]
    async fn test_login_success() {
        let mock_server = MockServer::start().await;

        // Mock the login page request
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(&mock_server)
            .await;

        // Mock the login form submission
        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .and(body_string_contains("UserName=test%40example.com"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server);
        let result = client.login("test@example.com", "password123").await;

        assert!(result.is_ok(), "Login should succeed: {:?}", result);
        assert!(client.is_logged_in());
    }

    #[tokio::test]
    async fn test_login_failure_invalid_credentials() {
        let mock_server = MockServer::start().await;

        // Mock the login page request
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(&mock_server)
            .await;

        // Mock the login form submission with failure response
        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_failed.html")))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server);
        let result = client.login("test@example.com", "wrong-password").await;

        assert!(result.is_err());
        match result {
            Err(KlafsError::AuthenticationFailed { .. }) => {}
            other => panic!("Expected AuthenticationFailed, got: {:?}", other),
        }
    }
}

mod status_tests {
    use super::*;

    async fn setup_logged_in_client(mock_server: &MockServer) -> KlafsClient {
        // Mock login
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(mock_server)
            .await;

        let client = create_test_client(mock_server);
        client
            .login("test@example.com", "password123")
            .await
            .unwrap();
        client
    }

    #[tokio::test]
    async fn test_get_status() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // Mock the status endpoint
        Mock::given(method("GET"))
            .and(path("/SaunaApp/GetData"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("sauna_status.json")))
            .mount(&mock_server)
            .await;

        let status = client
            .get_status("364cc9db-86f1-49d1-86cd-f6ef9b20a490")
            .await
            .unwrap();

        assert_eq!(status.sauna_id, "364cc9db-86f1-49d1-86cd-f6ef9b20a490");
        assert!(status.is_connected);
        assert!(!status.is_powered_on);
        assert!(status.sanarium_selected);
        assert_eq!(status.selected_sanarium_temperature, 70);
        assert_eq!(status.selected_hum_level, 7);
        assert_eq!(status.current_temperature, 22);
    }

    #[tokio::test]
    async fn test_get_status_powered_on() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/SaunaApp/GetData"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(fixture("sauna_status_powered_on.json")),
            )
            .mount(&mock_server)
            .await;

        let status = client
            .get_status("364cc9db-86f1-49d1-86cd-f6ef9b20a490")
            .await
            .unwrap();

        assert!(status.is_powered_on);
        assert!(status.sauna_selected);
        assert_eq!(status.status_code, 1); // Heating up
        assert_eq!(status.remaining_bathing_hours, 1);
        assert_eq!(status.remaining_bathing_minutes, 30);
    }

    #[tokio::test]
    async fn test_get_status_session_expired() {
        let mock_server = MockServer::start().await;
        let client = create_test_client(&mock_server);

        Mock::given(method("GET"))
            .and(path("/SaunaApp/GetData"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let result = client
            .get_status("364cc9db-86f1-49d1-86cd-f6ef9b20a490")
            .await;

        assert!(matches!(result, Err(KlafsError::SessionExpired)));
    }
}

mod list_saunas_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_saunas() {
        let mock_server = MockServer::start().await;

        // Setup login mocks
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(&mock_server)
            .await;

        // Mock the saunas list endpoint
        Mock::given(method("GET"))
            .and(path("/SaunaApp/ChangeSettings"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("saunas_list.html")))
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server);
        client
            .login("test@example.com", "password123")
            .await
            .unwrap();

        let saunas = client.list_saunas().await.unwrap();

        assert_eq!(saunas.len(), 2);
        assert_eq!(saunas[0].id, "364cc9db-86f1-49d1-86cd-f6ef9b20a490");
        assert_eq!(saunas[0].name, "Home Sauna");
        assert_eq!(saunas[1].id, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(saunas[1].name, "Guest House Sauna");
    }
}

mod power_control_tests {
    use super::*;

    async fn setup_logged_in_client(mock_server: &MockServer) -> KlafsClient {
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(mock_server)
            .await;

        let client = create_test_client(mock_server);
        client
            .login("test@example.com", "password123")
            .await
            .unwrap();
        client
    }

    #[tokio::test]
    async fn test_power_on_success() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/StartCabin"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("power_success.json")))
            .mount(&mock_server)
            .await;

        let result = client
            .power_on("364cc9db-86f1-49d1-86cd-f6ef9b20a490", "1234", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_power_off_success() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/StopCabin"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("power_success.json")))
            .mount(&mock_server)
            .await;

        let result = client
            .power_off("364cc9db-86f1-49d1-86cd-f6ef9b20a490")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_power_on_invalid_pin() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/StartCabin"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(fixture("power_invalid_pin.json")),
            )
            .mount(&mock_server)
            .await;

        // Use a valid 4-digit PIN format - server will reject it as wrong
        let result = client
            .power_on("364cc9db-86f1-49d1-86cd-f6ef9b20a490", "0000", None)
            .await;

        assert!(matches!(result, Err(KlafsError::InvalidPin)));
    }

    #[tokio::test]
    async fn test_power_on_scheduled() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // When scheduling, power_on first calls SetSelectedTime
        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetSelectedTime"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        // Then it calls StartCabin
        Mock::given(method("POST"))
            .and(path("/SaunaApp/StartCabin"))
            .and(body_string_contains("\"time_selected\":true"))
            .and(body_string_contains("\"sel_hour\":18"))
            .and(body_string_contains("\"sel_min\":30"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("power_success.json")))
            .mount(&mock_server)
            .await;

        let result = client
            .power_on(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                "1234",
                Some((18, 30)),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_power_on_schedule_validation() {
        let client = KlafsClient::new();

        // Invalid hour (too high)
        let result = client
            .power_on(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                "1234",
                Some((25, 30)),
            )
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid minute (too high)
        let result = client
            .power_on(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                "1234",
                Some((18, 60)),
            )
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }
}

mod control_tests {
    use super::*;

    async fn setup_logged_in_client(mock_server: &MockServer) -> KlafsClient {
        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(mock_server)
            .await;

        let client = create_test_client(mock_server);
        client
            .login("test@example.com", "password123")
            .await
            .unwrap();
        client
    }

    #[tokio::test]
    async fn test_set_mode() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetMode"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_mode("364cc9db-86f1-49d1-86cd-f6ef9b20a490", SaunaMode::Sauna)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_temperature() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/ChangeTemperature"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_temperature("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 85)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_temperature_invalid_range() {
        let client = KlafsClient::new();

        // Temperature too low
        let result = client
            .set_temperature("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 5)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Temperature too high
        let result = client
            .set_temperature("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 150)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_set_humidity() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // Mock status check (must be in Sanarium mode)
        Mock::given(method("GET"))
            .and(path("/SaunaApp/GetData"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("sauna_status.json")))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/ChangeHumLevel"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_humidity("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 7)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_humidity_not_in_sanarium_mode() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // Mock status check returning Sauna mode (not Sanarium)
        Mock::given(method("GET"))
            .and(path("/SaunaApp/GetData"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(fixture("sauna_status_sauna_mode.json")),
            )
            .mount(&mock_server)
            .await;

        let result = client
            .set_humidity("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 7)
            .await;

        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
        if let Err(KlafsError::InvalidParameter { message }) = result {
            assert!(message.contains("Sanarium mode"));
        }
    }

    #[tokio::test]
    async fn test_set_humidity_invalid_range() {
        let client = KlafsClient::new();

        // Level too low
        let result = client
            .set_humidity("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 0)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Level too high
        let result = client
            .set_humidity("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 11)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_set_start_time() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // set_start_time now delegates to set_selected_time which uses SetSelectedTime endpoint
        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetSelectedTime"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_start_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 18, 30)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_start_time_invalid() {
        let client = KlafsClient::new();

        // Invalid hour
        let result = client
            .set_start_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 25, 30)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid minute
        let result = client
            .set_start_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 18, 60)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_set_selected_time() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetSelectedTime"))
            .and(body_string_contains("\"time_set\":true"))
            .and(body_string_contains("\"hours\":18"))
            .and(body_string_contains("\"minutes\":30"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_selected_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", Some((18, 30)))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_selected_time_clear() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetSelectedTime"))
            .and(body_string_contains("\"time_set\":false"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .set_selected_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_selected_time_invalid() {
        let client = KlafsClient::new();

        // Invalid hour
        let result = client
            .set_selected_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", Some((25, 30)))
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid minute
        let result = client
            .set_selected_time("364cc9db-86f1-49d1-86cd-f6ef9b20a490", Some((18, 60)))
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_apply_favorite() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        Mock::given(method("POST"))
            .and(path("/SaunaApp/FavoriteSelected"))
            .and(body_string_contains("\"temp\":85"))
            .and(body_string_contains("\"hum_level\":5"))
            .and(body_string_contains("\"ir_level\":0"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .apply_favorite("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 85, 5, 0)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_apply_favorite_invalid() {
        let client = KlafsClient::new();

        // Invalid temperature (too low)
        let result = client
            .apply_favorite("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 5, 5, 0)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid temperature (too high)
        let result = client
            .apply_favorite("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 150, 5, 0)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid humidity level
        let result = client
            .apply_favorite("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 85, 15, 0)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));

        // Invalid IR level
        let result = client
            .apply_favorite("364cc9db-86f1-49d1-86cd-f6ef9b20a490", 85, 5, 15)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_configure() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // Mock the ChangeTemperature endpoint (configure now uses individual endpoints)
        Mock::given(method("POST"))
            .and(path("/SaunaApp/ChangeTemperature"))
            .and(body_string_contains("\"temperature\":85"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .configure(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                Some(85),
                None,
                None,
                None,
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_configure_with_time() {
        let mock_server = MockServer::start().await;
        let client = setup_logged_in_client(&mock_server).await;

        // Mock the SetSelectedTime endpoint
        Mock::given(method("POST"))
            .and(path("/SaunaApp/SetSelectedTime"))
            .and(body_string_contains("\"time_set\":true"))
            .and(body_string_contains("\"hours\":18"))
            .and(body_string_contains("\"minutes\":30"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"success": true}"#))
            .mount(&mock_server)
            .await;

        let result = client
            .configure(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                None,
                None,
                None,
                Some(18),
                Some(30),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_configure_no_changes() {
        let client = KlafsClient::new();

        // No parameters provided
        let result = client
            .configure(
                "364cc9db-86f1-49d1-86cd-f6ef9b20a490",
                None,
                None,
                None,
                None,
                None,
            )
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }
}

mod validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_pin_format_too_short() {
        let client = KlafsClient::new();
        let result = client
            .power_on("364cc9db-86f1-49d1-86cd-f6ef9b20a490", "123", None)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
        if let Err(KlafsError::InvalidParameter { message }) = result {
            assert!(message.contains("4 digits"));
        }
    }

    #[tokio::test]
    async fn test_invalid_pin_format_too_long() {
        let client = KlafsClient::new();
        let result = client
            .power_on("364cc9db-86f1-49d1-86cd-f6ef9b20a490", "12345", None)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_invalid_pin_format_non_numeric() {
        let client = KlafsClient::new();
        let result = client
            .power_on("364cc9db-86f1-49d1-86cd-f6ef9b20a490", "abcd", None)
            .await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_invalid_sauna_id_format() {
        let client = KlafsClient::new();

        // Test with garbage string
        let result = client.get_status("not-a-uuid").await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
        if let Err(KlafsError::InvalidParameter { message }) = result {
            assert!(message.contains("Invalid sauna ID format"));
        }
    }

    #[tokio::test]
    async fn test_invalid_sauna_id_empty() {
        let client = KlafsClient::new();
        let result = client.get_status("").await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }

    #[tokio::test]
    async fn test_invalid_sauna_id_partial_uuid() {
        let client = KlafsClient::new();
        let result = client.get_status("364cc9db-86f1-49d1").await;
        assert!(matches!(result, Err(KlafsError::InvalidParameter { .. })));
    }
}

mod debug_tests {
    use super::*;

    #[tokio::test]
    async fn test_debug_traffic_capture() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/Account/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_string(fixture("login_page.html")))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/Account/Login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(fixture("login_success.html"))
                    .insert_header("Set-Cookie", ".ASPXAUTH=test-auth-cookie; path=/"),
            )
            .mount(&mock_server)
            .await;

        let client = create_test_client(&mock_server);
        client
            .login("test@example.com", "password123")
            .await
            .unwrap();

        // Check that traffic was captured
        let traffic = client.debugger().get_traffic().await;
        assert!(
            traffic.len() >= 2,
            "Should have captured at least 2 requests"
        );

        // Verify first request (GET login page)
        assert_eq!(traffic[0].method, "GET");
        assert!(traffic[0].url.contains("/Account/Login"));

        // Verify second request (POST login)
        assert_eq!(traffic[1].method, "POST");
        assert!(traffic[1].url.contains("/Account/Login"));
    }
}
