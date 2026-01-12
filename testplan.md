# Manual Test Plan

Manual verification of the Klafs CLI against the real API.

**Prerequisites:**
- Valid Klafs account with registered sauna
- Sauna connected to network
- PIN for power control

---

## Authentication

- [ ] **Login with valid credentials**
  ```bash
  klafs login -u your@email.com
  ```
  - [ ] Password prompt appears
  - [ ] Success message displayed
  - [ ] Credentials stored in keyring

- [ ] **Login with invalid credentials**
  ```bash
  klafs login -u wrong@email.com
  ```
  - [ ] Error message displayed
  - [ ] No credentials stored

- [ ] **Logout**
  ```bash
  klafs logout
  ```
  - [ ] Success message displayed
  - [ ] Credentials removed from keyring

---

## Configuration

- [ ] **List saunas**
  ```bash
  klafs saunas
  ```
  - [ ] Shows registered sauna(s) with names and IDs

- [ ] **List saunas (JSON)**
  ```bash
  klafs saunas --json
  ```
  - [ ] Valid JSON output

- [ ] **Set default sauna ID**
  ```bash
  klafs config --sauna-id "your-sauna-uuid"
  ```
  - [ ] Success message displayed

- [ ] **Store PIN**
  ```bash
  klafs config --pin "1234"
  ```
  - [ ] Success message displayed
  - [ ] PIN stored in keyring

- [ ] **Show configuration**
  ```bash
  klafs config --show
  ```
  - [ ] Shows username, sauna ID, PIN status

---

## Status

- [ ] **Get status (human-readable)**
  ```bash
  klafs status
  ```
  - [ ] Shows connection status
  - [ ] Shows power status
  - [ ] Shows current mode
  - [ ] Shows temperatures
  - [ ] Shows humidity (if sanarium)

- [ ] **Get status (JSON)**
  ```bash
  klafs status --json
  ```
  - [ ] Valid JSON output with all fields

---

## Power Control

- [ ] **Power on immediately**
  ```bash
  klafs power-on
  ```
  - [ ] Success message displayed
  - [ ] Sauna begins heating (verify via status)

- [ ] **Power on with schedule**
  ```bash
  klafs power-on --at 18:30
  ```
  - [ ] Success message with scheduled time
  - [ ] Verify schedule via status

- [ ] **Power off**
  ```bash
  klafs power-off
  ```
  - [ ] Success message displayed
  - [ ] Sauna powers off (verify via status)

---

## Mode Control

- [ ] **Set mode to Sauna**
  ```bash
  klafs set-mode sauna
  ```
  - [ ] Success message displayed
  - [ ] Mode changed (verify via status)

- [ ] **Set mode to Sanarium**
  ```bash
  klafs set-mode sanarium
  ```
  - [ ] Success message displayed
  - [ ] Mode changed (verify via status)

- [ ] **Set mode to Infrared**
  ```bash
  klafs set-mode infrared
  ```
  - [ ] Success message displayed (or appropriate error if not supported)

---

## Temperature Control

- [ ] **Set temperature (valid range)**
  ```bash
  klafs set-temp 85
  ```
  - [ ] Success message displayed
  - [ ] Temperature changed (verify via status)

- [ ] **Set temperature (invalid - too low)**
  ```bash
  klafs set-temp 5
  ```
  - [ ] Error message displayed

- [ ] **Set temperature (invalid - too high)**
  ```bash
  klafs set-temp 150
  ```
  - [ ] Error message displayed

---

## Humidity Control

- [ ] **Set humidity level (in sanarium mode)**
  ```bash
  klafs set-humidity 7
  ```
  - [ ] Success message displayed
  - [ ] Humidity level changed (verify via status)

- [ ] **Set humidity level (invalid)**
  ```bash
  klafs set-humidity 15
  ```
  - [ ] Error message displayed

---

## Scheduling

- [ ] **Set schedule**
  ```bash
  klafs schedule 18:30
  ```
  - [ ] Success message displayed
  - [ ] Schedule set (verify via status)

- [ ] **Clear schedule**
  ```bash
  klafs schedule --clear
  ```
  - [ ] Success message displayed
  - [ ] Schedule cleared (verify via status)

---

## Configure (Combined)

- [ ] **Set temperature and humidity**
  ```bash
  klafs configure --temp 70 --humidity 5
  ```
  - [ ] Success message displayed
  - [ ] Both settings applied (verify via status)

- [ ] **Set temperature and schedule**
  ```bash
  klafs configure --temp 80 --time 19:00
  ```
  - [ ] Success message displayed
  - [ ] Both settings applied (verify via status)

---

## Profiles

- [ ] **Create profile (sauna)**
  ```bash
  klafs profile create hot --mode sauna --temp 90
  ```
  - [ ] Success message displayed

- [ ] **Create profile (sanarium)**
  ```bash
  klafs profile create relaxed --mode sanarium --temp 60 --humidity 7
  ```
  - [ ] Success message displayed

- [ ] **List profiles**
  ```bash
  klafs profile list
  ```
  - [ ] Shows created profiles with descriptions

- [ ] **Show profile details**
  ```bash
  klafs profile show hot
  ```
  - [ ] Shows mode, temperature, humidity

- [ ] **Apply profile**
  ```bash
  klafs profile apply hot
  ```
  - [ ] Success message displayed
  - [ ] Settings applied (verify via status)

- [ ] **Apply profile and start**
  ```bash
  klafs profile apply hot --start
  ```
  - [ ] Profile applied
  - [ ] Sauna started (verify via status)

- [ ] **Delete profile**
  ```bash
  klafs profile delete hot
  ```
  - [ ] Success message displayed
  - [ ] Profile removed (verify via list)

---

## Debug Mode

- [ ] **Enable debug logging**
  ```bash
  klafs --debug status
  ```
  - [ ] Debug output written to klafs-debug.log
  - [ ] Contains HTTP requests/responses

- [ ] **Custom debug file**
  ```bash
  klafs --debug --debug-file custom.log status
  ```
  - [ ] Debug output written to custom.log

---

## Error Handling

- [ ] **Invalid PIN**
  ```bash
  klafs power-on --pin 0000
  ```
  - [ ] Appropriate error message

- [ ] **Session expired**
  - [ ] Re-login prompt or clear error message

- [ ] **Network error**
  - [ ] Clear error message when offline

- [ ] **Sauna not connected**
  - [ ] Status shows disconnected state

---

## Notes

_Record any issues, unexpected behavior, or observations here:_

```
Date:
Tester:
Notes:



```
