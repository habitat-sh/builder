import { Component, OnInit, OnDestroy, AfterViewInit, inject, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { fromEvent, Subscription } from 'rxjs';
import { debounceTime } from 'rxjs/operators';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule
  ],
  template: `
    <footer 
      class="app-footer" 
      [class.sidebar-open]="sidebarOpen"
      [class.has-dynamic-position]="hasDynamicPosition">
      <div class="pull-left">
        Copyright © 2012-2025 Progress Software Corporation and/or its subsidiaries or affiliates. All Rights Reserved.
      </div>
      <span>
        Need help? If you have questions or you're stuck,
        <a href="https://www.chef.io/support" target="_blank">we're here to help</a>.
      </span>
      <div class="pull-right">
        <a href="https://www.chef.io/end-user-license-agreement" target="_blank">End User License Agreement</a>
        <a href="https://www.progress.com/legal/privacy-policy" target="_blank">Privacy Policy</a>
        <a href="https://www.progress.com/legal/cookie-policy" target="_blank">Cookie Policy</a>
      </div>
    </footer>
  `,
  styleUrls: ['../footer.component.scss'] // Point to the existing scss file
})
export class FooterComponent implements OnInit, AfterViewInit, OnDestroy {
  private changeDetector = inject(ChangeDetectorRef);
  
  sidebarOpen = true; // Default to true for desktop
  hasDynamicPosition = false; // Only enable when positioning is active
  private observer: MutationObserver | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private resizeSub: Subscription | null = null;
  
  ngOnInit() {
    // Initialize sidebar state immediately and then set up observers
    this.checkSidebarState();
    setTimeout(() => {
      this.setupSidebarObserver();
      this.setupResizeObserver();
    }, 100); // Slight delay to ensure DOM is ready
  }
  
  ngOnDestroy() {
    if (this.observer) {
      this.observer.disconnect();
      this.observer = null;
    }
    
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
    
    if (this.resizeSub) {
      this.resizeSub.unsubscribe();
      this.resizeSub = null;
    }
  }
  
  // Check the sidebar state more aggressively
  private checkSidebarState() {
    const sidebar = document.querySelector('app-sidebar.menu');
    
    // Default to true on tablet and desktop (matches CSS media query)
    const isTabletOrLarger = window.innerWidth >= 768; // Must match the mixins breakpoint
    
    if (sidebar) {
      // First check if sidebar has explicit state
      if (sidebar.classList.contains('open')) {
        this.sidebarOpen = true;
      } else if (sidebar.classList.contains('open') === false) {
        // Only set to false if explicitly not open class
        this.sidebarOpen = false;
      } else {
        // Otherwise use the screen size as a guide
        this.sidebarOpen = isTabletOrLarger;
      }
    } else {
      // No sidebar found, use screen size
      this.sidebarOpen = isTabletOrLarger;
    }
    
    console.log('Sidebar state:', this.sidebarOpen, 'Screen width:', window.innerWidth);
    this.changeDetector.detectChanges();
  }
  
  private setupSidebarObserver() {
    const sidebar = document.querySelector('app-sidebar.menu');
    if (sidebar) {
      // Double-check state when setting up observer - default to true for tablet+
      const isTabletOrLarger = window.innerWidth >= 768;
      
      // First check if sidebar has a specific open class
      if (sidebar.classList.contains('open')) {
        this.sidebarOpen = true;
      } else if (sidebar.classList.contains('open') === false && !isTabletOrLarger) {
        // Only set to false if explicitly not open AND mobile view
        this.sidebarOpen = false;
      } else {
        // For tablet+ default to true when not explicitly set
        this.sidebarOpen = isTabletOrLarger;
      }
      
      console.log('Setting up sidebar observer with state:', this.sidebarOpen);
      
      // Set up mutation observer to track sidebar open/closed state
      this.observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
          if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
            const target = mutation.target as Element;
            const isOpen = target.classList.contains('open');
            console.log('Sidebar class changed:', isOpen);
            
            // If explicitly open or screen is large enough, set to true
            if (isOpen || isTabletOrLarger) {
              this.sidebarOpen = true;
            } else {
              // Otherwise use the class value
              this.sidebarOpen = isOpen;
            }
            
            this.changeDetector.detectChanges();
          }
        });
      });
      
      this.observer.observe(sidebar, { attributes: true, attributeFilter: ['class'] });
    }
  }
  
  // Set up an observer to track main content position changes
  private setupResizeObserver() {
    // Check if ResizeObserver is available
    if (typeof ResizeObserver === 'undefined') {
      return;
    }
    
    const main = document.querySelector('app-shell main');
    const sidebar = document.querySelector('app-sidebar.menu');
    
    if (main) {
      // Initial adjustment
      this.adjustFooterPosition(main, sidebar);
      
      // Create ResizeObserver to track changes
      this.resizeObserver = new ResizeObserver(() => {
        this.adjustFooterPosition(main, sidebar);
      });
      
      this.resizeObserver.observe(main);
      
      // Also observe the sidebar if it exists
      if (sidebar) {
        this.resizeObserver.observe(sidebar);
      }
    }
  }
  
  // Adjust footer CSS variables based on main content position
  private adjustFooterPosition(main: Element, sidebar?: Element | null) {
    const mainRect = main.getBoundingClientRect();
    const footer = document.querySelector('.app-footer') as HTMLElement;
    
    if (footer) {
      let leftPosition = 0;
      
      // First try to use the sidebar width if available
      if (sidebar && window.innerWidth >= 768) {
        const sidebarRect = sidebar.getBoundingClientRect();
        // Only use the sidebar position if it's visible (not transformed offscreen)
        if (sidebarRect.right > 0 && this.sidebarOpen) {
          leftPosition = sidebarRect.width;
          console.log('Using sidebar width:', leftPosition);
        }
      }
      
      // Fallback to main content left position
      if (leftPosition === 0 && mainRect.left > 0) {
        leftPosition = mainRect.left;
        console.log('Using main content left:', leftPosition);
      }
      
      // Apply position if we have a valid value
      if (leftPosition > 0) {
        // The left position determines where our footer should start
        footer.style.setProperty('--main-left-position', `${leftPosition}px`);
        
        // Enable dynamic positioning after we've set the variable
        if (!this.hasDynamicPosition) {
          this.hasDynamicPosition = true;
          this.changeDetector.detectChanges();
        }
      } else {
        // If we can't get a valid position, make sure we use the static positioning
        console.log('No valid position detected, using static CSS');
        this.hasDynamicPosition = false;
        this.changeDetector.detectChanges();
      }
    }
  }
  
  ngAfterViewInit() {
    // Set up a window resize listener for responsive handling
    this.resizeSub = fromEvent(window, 'resize')
      .pipe(debounceTime(150))
      .subscribe(() => {
        this.checkSidebarState();
        const main = document.querySelector('app-shell main');
        const sidebar = document.querySelector('app-sidebar.menu');
        if (main) {
          this.adjustFooterPosition(main, sidebar);
        }
        this.changeDetector.detectChanges();
      });
  }
}
