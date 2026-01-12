# Manual Test Plan

Manual verification of the sauna CLI against the real Klafs API.

**Prerequisites:**
- Valid Klafs account with registered sauna
- Sauna connected to network
- PIN for power control

---

## Authentication

- [ ] **Login with valid credentials**
  ```bash
  sauna login -u your@email.com
  ```
  - [ ] Password prompt appears
  - [ ] Success message displayed
  - [ ] Credentials stored in keyring

- [ ] **Login with invalid credentials**
  ```bash
  sauna login -u wrong@email.com
  ```
  - [ ] Error message displayed
  - [ ] No credentials stored

- [ ] **Logout**
  ```bash
  sauna logout
  ```
  - [ ] Success message displayed
  - [ ] Credentials removed from keyring

---

## Configuration

- [ ] **List saunas**
  ```bash
  sauna saunas
  ```
  - [ ] Shows registered sauna(s) with names and IDs

- [ ] **List saunas (JSON)**
  ```bash
  sauna saunas --json
  ```
  - [ ] Valid JSON output

- [ ] **Set default sauna ID**
  ```bash
  sauna config --sauna-id "your-sauna-uuid"
  ```
  - [ ] Success message displayed

- [ ] **Store PIN**
  ```bash
  sauna config --pin "1234"
  ```
  - [ ] Success message displayed
  - [ ] PIN stored in keyring

- [ ] **Show configuration**
  ```bash
  sauna config --show
  ```
  - [ ] Shows username, sauna ID, PIN status

---

## Status

- [ ] **Get status (human-readable)**
  ```bash
  sauna status
  ```
  - [ ] Shows connection status
  - [ ] Shows power status
  - [ ] Shows current mode
  - [ ] Shows temperatures
  - [ ] Shows humidity (if sanarium)

- [ ] **Get status (JSON)**
  ```bash
  sauna status --json
  ```
  - [ ] Valid JSON output with all fields

---

## Power Control

- [ ] **Power on immediately**
  ```bash
  sauna power-on
  ```
  - [ ] Success message displayed
  - [ ] Sauna begins heating (verify via status)

- [ ] **Power on with schedule**
  ```bash
  sauna power-on --at 18:30
  ```
  - [ ] Success message with scheduled time
  - [ ] Verify schedule via status

- [ ] **Power off**
  ```bash
  sauna power-off
  ```
  - [ ] Success message displayed
  - [ ] Sauna powers off (verify via status)

---

## Mode Control

- [ ] **Set mode to Sauna**
  ```bash
  sauna set-mode sauna
  ```
  - [ ] Success message displayed
  - [ ] Mode changed (verify via status)

- [ ] **Set mode to Sanarium**
  ```bash
  sauna set-mode sanarium
  ```
  - [ ] Success message displayed
  - [ ] Mode changed (verify via status)

- [ ] **Set mode to Infrared**
  ```bash
  sauna set-mode infrared
  ```
  - [ ] Success message displayed (or appropriate error if not supported)

---

## Temperature Control

- [ ] **Set temperature (valid range)**
  ```bash
  sauna set-temp 85
  ```
  - [ ] Success message displayed
  - [ ] Temperature changed (verify via status)

- [ ] **Set temperature (invalid - too low)**
  ```bash
  sauna set-temp 5
  ```
  - [ ] Error message displayed

- [ ] **Set temperature (invalid - too high)**
  ```bash
  sauna set-temp 150
  ```
  - [ ] Error message displayed

---

## Humidity Control

- [ ] **Set humidity level (in sanarium mode)**
  ```bash
  sauna set-humidity 7
  ```
  - [ ] Success message displayed
  - [ ] Humidity level changed (verify via status)

- [ ] **Set humidity level (invalid)**
  ```bash
  sauna set-humidity 15
  ```
  - [ ] Error message displayed

---

## Scheduling

- [ ] **Set schedule**
  ```bash
  sauna schedule 18:30
  ```
  - [ ] Success message displayed
  - [ ] Schedule set (verify via status)

- [ ] **Clear schedule**
  ```bash
  sauna schedule --clear
  ```
  - [ ] Success message displayed
  - [ ] Schedule cleared (verify via status)

---

## Configure (Combined)

- [ ] **Set temperature and humidity**
  ```bash
  sauna configure --temp 70 --humidity 5
  ```
  - [ ] Success message displayed
  - [ ] Both settings applied (verify via status)

- [ ] **Set temperature and schedule**
  ```bash
  sauna configure --temp 80 --time 19:00
  ```
  - [ ] Success message displayed
  - [ ] Both settings applied (verify via status)

---

## Profiles

- [ ] **Create profile (sauna)**
  ```bash
  sauna profile create hot --mode sauna --temp 90
  ```
  - [ ] Success message displayed

- [ ] **Create profile (sanarium)**
  ```bash
  sauna profile create relaxed --mode sanarium --temp 60 --humidity 7
  ```
  - [ ] Success message displayed

- [ ] **List profiles**
  ```bash
  sauna profile list
  ```
  - [ ] Shows created profiles with descriptions

- [ ] **Show profile details**
  ```bash
  sauna profile show hot
  ```
  - [ ] Shows mode, temperature, humidity

- [ ] **Apply profile**
  ```bash
  sauna profile apply hot
  ```
  - [ ] Success message displayed
  - [ ] Settings applied (verify via status)

- [ ] **Apply profile and start**
  ```bash
  sauna profile apply hot --start
  ```
  - [ ] Profile applied
  - [ ] Sauna started (verify via status)

- [ ] **Delete profile**
  ```bash
  sauna profile delete hot
  ```
  - [ ] Success message displayed
  - [ ] Profile removed (verify via list)

---

## Debug Mode

- [ ] **Enable debug logging**
  ```bash
  sauna --debug status
  ```
  - [ ] Debug output written to klafs-debug.log
  - [ ] Contains HTTP requests/responses

- [ ] **Custom debug file**
  ```bash
  sauna --debug --debug-file custom.log status
  ```
  - [ ] Debug output written to custom.log

---

## Error Handling

- [ ] **Invalid PIN**
  ```bash
  sauna power-on --pin 0000
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
