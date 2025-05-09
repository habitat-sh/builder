# Footer Component Layout

## Overview

The footer component in our application has been designed to properly align with the main content area and respect the sidebar layout. This ensures the footer starts after the sidebar on desktop, but spans the full width on mobile devices.

## Implementation Details

### CSS Structure

The footer uses the following key CSS properties:

```scss
.app-footer {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  width: 100%;
  
  // Default footer positioning (sidebar closed)
  @include mix.tablet-up {
    left: 0;
    width: 100%;
  }
  
  // When sidebar is open, adjust the footer position
  &.sidebar-open {
    @include mix.tablet-up {
      left: $menu-width; 
      width: calc(100% - #{$menu-width});
    }
  }
}
```

### Dynamic Positioning Using CSS Variables

The footer also uses CSS variables to dynamically position itself based on the main content area:

```scss
.app-footer {
  @include mix.tablet-up {
    // Use CSS variables with fallback - set dynamically by JavaScript
    left: var(--main-left-position, $menu-width);
    width: calc(100% - var(--main-left-position, #{$menu-width}));
  }
}
```

### Responsive Behavior

- **Desktop with sidebar open**: The footer begins after the sidebar (224px from the left) to align with the main content area.
- **Desktop with sidebar closed**: The footer spans the full width of the screen.
- **Mobile**: The footer spans the full width of the screen and has simplified content.

### Sidebar State Detection

The footer component uses multiple approaches to detect sidebar state:

1. **MutationObserver** to track sidebar class changes:

```typescript
private setupSidebarObserver() {
  const sidebar = document.querySelector('app-sidebar.menu');
  if (sidebar) {
    // Double-check state when setting up observer
    this.sidebarOpen = sidebar.classList.contains('open');
    
    // Set up mutation observer to track sidebar open/closed state
    this.observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
          const isOpen = (mutation.target as Element).classList.contains('open');
          console.log('Sidebar state changed:', isOpen);
          this.sidebarOpen = isOpen;
        }
      });
    });
    
    this.observer.observe(sidebar, { attributes: true, attributeFilter: ['class'] });
  }
}
```

2. **ResizeObserver** to adjust footer position when layout changes:

```typescript
private setupResizeObserver() {
  // Check if ResizeObserver is available
  if (typeof ResizeObserver === 'undefined') {
    return;
  }
  
  const main = document.querySelector('app-shell main');
  if (main) {
    // Initial adjustment
    this.adjustFooterPosition(main);
    
    // Create ResizeObserver to track changes
    this.resizeObserver = new ResizeObserver(() => {
      this.adjustFooterPosition(main);
    });
    
    this.resizeObserver.observe(main);
  }
}

// Adjust footer CSS variables based on main content position
private adjustFooterPosition(main: Element) {
  const rect = main.getBoundingClientRect();
  const footer = document.querySelector('.app-footer') as HTMLElement;
  
  if (footer) {
    // The left position of the main content determines where our footer should start
    footer.style.setProperty('--main-left-position', `${rect.left}px`);
  }
}
```

3. **Responsive window resize handler** for layout changes:

```typescript
ngAfterViewInit() {
  // Set up a window resize listener for responsive handling
  this.resizeSub = fromEvent(window, 'resize')
    .pipe(debounceTime(150))
    .subscribe(() => {
      this.checkSidebarState();
      const main = document.querySelector('app-shell main');
      if (main) {
        this.adjustFooterPosition(main);
      }
      this.changeDetector.detectChanges();
    });
}
```

### Layout integration

The footer is a child of the `<main>` element in the app-shell component:

```html
<main>
  <app-header></app-header>
  <div class="content-container">
    <router-outlet></router-outlet>
  </div>
  <app-footer></app-footer>
</main>
```

This structure ensures that the footer is properly positioned relative to the main content area.

## Best Practices

1. Use fixed positioning for the footer to ensure it stays at the bottom of the screen
2. Implement multiple detection strategies for the most reliable positioning
3. Use CSS variables for dynamic positioning that adapts to layout changes
4. Leverage responsive CSS to handle different screen sizes
5. Clean up all observers and subscriptions in ngOnDestroy to prevent memory leaks
