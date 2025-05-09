# Image Loading Strategy

To ensure reliable loading of images and proper fallbacks, the application implements several strategies:

## Asset Management

Images and other static assets are stored in the following locations:
- Primary location: `src/assets/images/`
- Secondary fallback: `public/` directory (configured in angular.json)

## Fallback Mechanisms

The application implements a multi-layered fallback mechanism for image loading:

1. **FallbackImageDirective**: A custom directive that handles image loading errors by:
   - Trying alternative paths
   - Using specified fallback images 
   - Creating dynamic fallback elements with CSS styling

   Usage:
   ```html
   <img [src]="logoUrl" habFallbackImage fallbackType="logo">
   ```

2. **Image Loader Script**: A global script that preloads critical assets and fixes paths at runtime
   - Preloads key images to determine which paths work
   - Dynamically replaces broken image paths
   - Provides CSS fallbacks when images cannot be loaded

3. **AssetLoaderService**: Angular service for tracking image loading success/failure
   - Provides diagnostic information on which images fail to load
   - Helps identify patterns in image loading issues

## Debug Tools

For troubleshooting image loading issues:

1. Navigate to `/debug/assets` to see the asset loading diagnostic page
2. Check browser console for image loading errors and fixes
3. Use the Network tab in DevTools to check image requests

## Adding New Images

When adding new images:

1. Add them to `src/assets/images/`
2. Test with the `habFallbackImage` directive for automatic fallback handling
3. For critical UI elements, consider adding to the preloaded images list in the image-loader.js script
