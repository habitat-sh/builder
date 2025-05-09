# Font Implementation Documentation

This document describes the font implementation in the Habitat Builder UI, which has been designed to match the original builder-web application.

## Font Families

The application uses two primary font families:

1. **Base Font**: `'Titillium Web', 'Helvetica Neue', Helvetica, Roboto, Arial, sans-serif`
   - Used for general text content throughout the application
   - Applied to body text, navigation items, and most UI elements

2. **Heading Font**: `Montserrat, Verdana`
   - Used for section headings, especially in the sidebar
   - Provides visual hierarchy and distinguished style for headings

## Sidebar Dimensions and Spacing

To ensure visual consistency with the original application, the following sidebar dimensions and spacing were implemented:

1. **Sidebar Width**: Fixed at 224px, matching the original builder-web implementation
2. **Horizontal Padding**: 16px top/bottom with 32px left/right padding
3. **Icon Sizing**: Icons have consistent 20px dimensions with 8px right margin
4. **Navigation Group Spacing**: Reduced padding for nested navigation items
5. **Headings Spacing**: 24px top margin (except first heading) and 8px bottom margin

These precise measurements ensure that the sidebar appears identical in width and spacing to the original application.

## Font Files

The font files are stored in the `/assets/fonts/` directory and include:

- `titilliumweb-regular.ttf` - Regular weight (400)
- `titilliumweb-italic.ttf` - Italic style (400)
- `titilliumweb-semibold.ttf` - Semi-bold weight (600)
- `titilliumweb-bold.ttf` - Bold weight (700)
- `montserrat-regular.ttf` - Regular weight (400)

## Implementation Details

Font declarations are defined in `/src/app/core/styles/_fonts.scss` which includes the `@font-face` declarations for all font variations.

Typography variables are defined in `/src/app/core/styles/_typography.scss`:

```scss
$base-font-family: 'Titillium Web', 'Helvetica Neue', Helvetica, Roboto, Arial, sans-serif;
$heading-font-family: Montserrat, Verdana;
$monospace-font-family: 'Source Code Pro', Menlo, Consolas, monospace;
$tabs-font-family: $heading-font-family;
```

These variables are then used throughout the application to ensure consistent typography.

## Sidebar-Specific Font Styling

The sidebar component has specific font styling to match the original builder-web application:

- Section headings use `Montserrat` font with uppercase text and letter spacing
- Navigation items use `Titillium Web` with specific font weights and sizes

## Migration Notes

This font implementation was part of the Angular 19 migration project, ensuring visual consistency with the original Angular 6 implementation. The fonts were copied from the original builder-web application to maintain the exact same appearance.
