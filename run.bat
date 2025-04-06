:: STARTS BOTH PROJECTS AT ONCE, SAFE TO RUN TRUST

@echo off

REM Start first cargo project in a new window
start cmd /k "cd actuator_proj && cargo run"

REM Start second cargo project in a new window
start cmd /k "cd sensor_proj && cargo run"