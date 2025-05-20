# Real Time Arm Simulation
Student A - Controls the arms
- Should be controlling some simulated arm-actuator/motor
- Acts on feedback from student B
- Adjusts the sensor processing based on feedback from student B (e.g., recalibrating threshold values or refining anomaly detection).

Student B - Controller
- Should sent instructons for the arm to act on
- Received sensor data from student A within 1ms that will be acted upon
- Make sure actuators respond within 1-2ms

DEADLINE THRESHOLDS:
-	Ensure data is generated at fixed intervals (e.g., every 5ms) without excessive jitter.
- Ensure that data reception occurs within 1ms of transmission from Student A.
- Meet strict deadlines—data must be transmitted within 1ms after processing (data from student B to A).
- Ensure feedback transmission completes within 1ms of actuation execution.

Evaluation Criteria
Your project will be assessed based on the following:
•	Correctness – Does each component function as intended?
•	Real-Time Performance – Are response times predictable and within deadlines?
•	Efficiency – Are Rust’s strengths (such as memory safety and concurrency) used effectively?
•	Interoperability – How seamlessly do the two components exchange data?
•	Scalability – Can the system handle multiple actuators and dynamic conditions?
•	Code Quality & Documentation – Is the Rust code well-structured and documented?
•	Performance Benchmarking – How thorough is your performance analysis?
