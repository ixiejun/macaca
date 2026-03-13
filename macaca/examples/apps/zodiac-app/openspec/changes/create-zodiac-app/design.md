# Design Document

## Context
Building a zodiac query web application as a standalone Next.js application. Users will input their birth date (month/day) to discover their zodiac sign and associated information.

## Goals / Non-Goals

### Goals
- Beautiful, responsive UI with zodiac theme
- Fast and accurate zodiac calculation
- Clean API design
- Great user experience with loading/error states

### Non-Goals
- User authentication
- Persistent data storage
- Multi-language support (initial version)
- Horoscope predictions API

## Decisions

### Tech Stack
- **Framework**: Next.js 14 with App Router
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **UI Approach**: Custom components with zodiac-inspired design

### Architecture
```
/src
  /app
    /api/zodiac/route.ts  # API endpoint
    page.tsx              # Main page
    layout.tsx            # Root layout
  /components
    DateSelector.tsx      # Date input component
    ZodiacCard.tsx        # Result display component
  /lib
    zodiac.ts             # Zodiac calculation logic
    constants.ts          # Zodiac data constants
```

### API Design
- `GET /api/zodiac?month={month}&day={day}`
- Returns: `{ sign, name, dateRange, element, traits, luckyNumbers, luckyColors, ... }`

## Risks / Trade-offs
- No external API dependency (all data is static) → Simpler but less dynamic content
- Client-side date validation only → Sufficient for this use case
