/**
 * This script fixes image loading issues by preloading key assets 
 * and providing fallbacks when they fail to load.
 */

// Images that need to be preloaded
const criticalImages = [
  'assets/images/habitat-logo.svg',
  '/assets/images/habitat-logo.svg',
  'assets/images/builder-habitat-logo.svg',
  '/assets/images/builder-habitat-logo.svg',
  'assets/images/avatar.svg',
  '/assets/images/avatar.svg',
  'assets/images/habicat.svg',
  '/assets/images/habicat.svg'
];

// Keep track of which images loaded successfully
const loadedImages = new Map();
const failedImages = new Map();

// Function to preload images
function preloadImages() {
  console.log('[Image Loader] Preloading critical images...');
  
  criticalImages.forEach(src => {
    const img = new Image();
    
    img.onload = () => {
      console.log(`[Image Loader] Successfully loaded: ${src}`);
      loadedImages.set(src, true);
    };
    
    img.onerror = () => {
      console.error(`[Image Loader] Failed to load: ${src}`);
      failedImages.set(src, true);
    };
    
    img.src = src;
  });
}

// Function to find a working version of an image
function findWorkingImagePath(originalPath) {
  // If the original path works, use it
  if (loadedImages.has(originalPath)) {
    return originalPath;
  }
  
  // Try to find an alternative path that worked
  const fileName = originalPath.split('/').pop();
  if (!fileName) return null;
  
  for (const [path, loaded] of loadedImages.entries()) {
    if (path.includes(fileName)) {
      return path;
    }
  }
  
  // Find any habitat logo that loaded
  if (originalPath.includes('habitat-logo') || originalPath.includes('builder-habitat-logo')) {
    for (const [path, loaded] of loadedImages.entries()) {
      if (path.includes('habitat-logo')) {
        return path;
      }
    }
  }
  
  // Return null if no suitable replacement found
  return null;
}

// Add a global helper to fix image paths
window.fixImageSrc = function(img) {
  const originalSrc = img.getAttribute('src');
  const workingPath = findWorkingImagePath(originalSrc);
  
  if (workingPath && workingPath !== originalSrc) {
    console.log(`[Image Loader] Replacing ${originalSrc} with working path ${workingPath}`);
    img.setAttribute('src', workingPath);
    return true;
  }
  
  return false;
};

// Create CSS classes for fallback text logos
function addFallbackStyles() {
  const style = document.createElement('style');
  style.textContent = `
    .habitat-logo-fallback {
      width: 36px;
      height: 36px;
      background-color: #FF9012;
      border-radius: 4px;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: bold;
      font-size: 20px;
    }
    
    .avatar-fallback {
      width: 32px;
      height: 32px;
      background-color: #607D8B;
      border-radius: 50%;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 18px;
    }
  `;
  document.head.appendChild(style);
}

// Main initialization
function initImageLoader() {
  console.log('[Image Loader] Initializing...');
  preloadImages();
  addFallbackStyles();
  
  // Add a global error handler for all images
  document.addEventListener('error', function(event) {
    if (event.target.tagName.toLowerCase() === 'img') {
      const img = event.target;
      const src = img.getAttribute('src');
      console.error(`[Image Loader] Runtime error loading image: ${src}`);
      
      // Try to fix the image
      if (!window.fixImageSrc(img)) {
        // If no working path found, replace with fallback
        const container = img.parentNode;
        if (container) {
          if (src.includes('habitat-logo') || src.includes('builder-habitat-logo')) {
            const fallback = document.createElement('div');
            fallback.className = 'habitat-logo-fallback';
            fallback.textContent = 'H';
            container.replaceChild(fallback, img);
          } else if (src.includes('avatar')) {
            const fallback = document.createElement('div');
            fallback.className = 'avatar-fallback';
            fallback.textContent = 'U';
            container.replaceChild(fallback, img);
          }
        }
      }
    }
  }, true);
}

// Run the initializer
document.addEventListener('DOMContentLoaded', initImageLoader);
console.log('[Image Loader] Script loaded');
