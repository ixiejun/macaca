# Zodiac Query Capability

## ADDED Requirements

### Requirement: Zodiac Sign Calculation
The system SHALL calculate the correct zodiac sign based on a user's birth month and day.

#### Scenario: Valid date input
- **WHEN** user provides a valid month (1-12) and day (1-31)
- **THEN** the system returns the corresponding zodiac sign

#### Scenario: Leap year handling
- **WHEN** user provides February 29 as birth date
- **THEN** the system returns Pisces as the zodiac sign

#### Scenario: Cusp date handling
- **WHEN** user provides a date on a sign boundary (e.g., March 21)
- **THEN** the system returns the correct sign (Aries for March 21)

### Requirement: Zodiac Information Display
The system SHALL display comprehensive information about each zodiac sign.

#### Scenario: Display zodiac details
- **WHEN** a zodiac sign is determined
- **THEN** the system displays:
  - Sign name and symbol
  - Date range
  - Element (Fire, Earth, Air, Water)
  - Ruling planet
  - Personality traits
  - Lucky numbers
  - Lucky colors
  - Compatibility signs

### Requirement: Date Input Interface
The system SHALL provide an intuitive interface for users to input their birth date.

#### Scenario: Date selection
- **WHEN** user accesses the application
- **THEN** they can select month and day separately

#### Scenario: Input validation
- **WHEN** user enters an invalid date (e.g., February 30)
- **THEN** the system displays an appropriate error message

### Requirement: Responsive Design
The system SHALL provide a responsive interface that works across device sizes.

#### Scenario: Mobile display
- **WHEN** user accesses the application on a mobile device
- **THEN** the interface adapts to the smaller screen size

#### Scenario: Desktop display
- **WHEN** user accesses the application on a desktop
- **THEN** the interface takes advantage of larger screen real estate

### Requirement: Loading and Error States
The system SHALL provide visual feedback during loading and error conditions.

#### Scenario: Loading state
- **WHEN** a zodiac query is in progress
- **THEN** a loading indicator is displayed

#### Scenario: Error handling
- **WHEN** an error occurs
- **THEN** a user-friendly error message is displayed
