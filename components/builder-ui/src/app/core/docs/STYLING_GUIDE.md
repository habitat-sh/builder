# Habitat Builder UI Styling Guide

This document provides an overview of the styling approach used in the Habitat Builder UI application, which has been migrated from Angular 6 to Angular 19.

## Core Principles

The UI follows these core design principles:
- Clean, modern interface with a focus on usability
- Consistent visual language across all components
- Responsive design for various screen sizes
- Proper hierarchy and visual feedback for interactive elements

## Color System

The application uses a defined color palette from the Habitat design system:

### Primary Colors
- `$hab-blue`: #3292bf - Primary brand color
- `$hab-orange`: #ff9600 - Accent/highlight color
- `$dark-blue`: #283c4f - Used for headers and main UI elements
- `$white`: #ffffff - Used for backgrounds and text on dark surfaces

### Secondary Colors
- `$medium-blue`: #556F84 - Secondary text, icons
- `$light-gray`: #CFD7DE - Borders, dividers
- `$very-light-gray`: #EFF4F7 - Background for secondary elements
- `$off-white`: #FAFBFC - Background for main content areas

### Text Colors
- `$hab-text`: #4D6070 - Primary text color
- `$dark-gray`: #444444 - Secondary text color

## Typography

The application uses a simple, readable type system:

- Base font: 'Open Sans', 'Helvetica Neue', sans-serif
- Heading font: 'Titillium Web', 'Open Sans', sans-serif (for titles and headers)
- Monospace font: 'Source Code Pro', Consolas, monospace (for code and terminal output)

### Type Scale
- `$small-font-size`: 13px
- `$base-font-size`: 15px
- `$large-font-size`: 18px
- `$base-line-height`: 1.5

## Components

### Cards
Cards are used extensively throughout the UI to group related content. They feature:
- Subtle shadow and border
- Consistent padding
- Optional accent border at the top (created with gradient)
- Hover animations for interactive cards

### Buttons
- Primary buttons use `$hab-blue` background
- Secondary/outlined buttons have a light border
- All buttons feature hover/active state animations
- Text buttons have subtle hover states

### Navigation
- The sidebar uses a dark background with light text
- Active navigation items are highlighted with an orange accent
- The header uses a light background with dark text for contrast

## Angular Material Theme

A custom Angular Material theme has been created to match the Habitat design system. This replaces the default prebuilt themes and ensures consistent styling across all Material components.

The theme is defined in `src/theme.scss` and includes:
- Custom color palettes for primary, accent, and warning colors
- Typography configuration
- Component-specific overrides

## SCSS Structure

The application uses SCSS for styling with the following organization:

1. **Core Styles**: Located in `src/app/core/styles/`
   - `_colors.scss`: Color variables 
   - `_typography.scss`: Font families, sizes, and weights
   - `_mixins.scss`: Reusable style patterns and responsive mixins

2. **Component Styles**: Each component has its own `.scss` file with styles scoped to that component

3. **Global Styles**: Located in `src/styles.scss` with global rules and Material theme imports

## Responsive Design

The UI is designed to be responsive across different screen sizes:

- Desktop (1024px and up): Full layout with sidebar
- Tablet (768px to 1024px): Compressed layout
- Mobile (below 768px): Stacked elements, hidden sidebar with toggle

Responsive breakpoints are defined in the mixins and used consistently throughout the application:

```scss
@include mix.desktop-up { /* styles for desktop */ }
@include mix.tablet-up { /* styles for tablets and up */ }
@include mix.mobile { /* styles for mobile */ }
```

## Best Practices

When modifying or creating components:

1. Use the predefined color variables instead of hardcoded values
2. Follow the established component patterns for consistency
3. Test on multiple screen sizes to ensure proper responsive behavior
4. Use animations sparingly and with purpose
5. Ensure sufficient contrast for accessibility
6. Use Material components when possible, with custom styles as needed

## Animation Guidelines

The UI uses subtle animations to enhance the user experience:

- Transition duration: 0.2s to 0.3s
- Easing: ease or ease-in-out for most transitions
- Hover states: Slight scaling, shadow changes, or color transitions
- Active states: Typically reverse or reduce the hover effect

## Future Improvements

Ongoing styling improvements planned for the UI include:

1. Further refinement of Material component overrides
2. Creation of reusable custom components
3. Enhanced dark mode support
4. Accessibility improvements
5. Animation library for more complex transitions
