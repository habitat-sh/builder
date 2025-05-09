import { Directive, ElementRef, HostListener, Input, Renderer2 } from '@angular/core';

/**
 * Directive to handle image loading errors with fallback mechanisms
 * Usage: <img habFallbackImage [fallbackSrc]="'path/to/fallback.jpg'" [fallbackType]="'logo'">
 */
@Directive({
  selector: 'img[habFallbackImage]',
  standalone: true
})
export class FallbackImageDirective {
  @Input() fallbackSrc: string = '';
  @Input() fallbackType: 'logo' | 'avatar' | 'default' = 'default';
  
  private originalSrc: string = '';
  private hasFailed: boolean = false;
  
  constructor(private el: ElementRef, private renderer: Renderer2) {}
  
  ngOnInit() {
    this.originalSrc = this.el.nativeElement.src;
  }
  
  @HostListener('error')
  onError() {
    if (this.hasFailed) {
      this.replaceMissingImageWithFallback();
      return;
    }
    
    this.hasFailed = true;
    
    // Try to use the window.fixImageSrc function if available
    const windowObj = window as any;
    if (windowObj.fixImageSrc && windowObj.fixImageSrc(this.el.nativeElement)) {
      return;
    }
    
    // If specific fallback source is provided, try it first
    if (this.fallbackSrc) {
      this.renderer.setAttribute(this.el.nativeElement, 'src', this.fallbackSrc);
    } 
    // Otherwise try a calculated fallback based on original source
    else {
      const imgElement = this.el.nativeElement;
      const currentSrc = imgElement.src;
      
      // Try alternative paths
      if (currentSrc.includes('builder-habitat-logo')) {
        this.renderer.setAttribute(imgElement, 'src', 'assets/images/habitat-logo.svg');
      } 
      else if (currentSrc.includes('/assets/') && !currentSrc.startsWith('/assets/')) {
        this.renderer.setAttribute(imgElement, 'src', '/' + currentSrc);
      }
      else if (!currentSrc.includes('/assets/') && !currentSrc.startsWith('/assets/')) {
        this.renderer.setAttribute(imgElement, 'src', '/assets/images/' + currentSrc.split('/').pop());
      }
      // If using avatar, try the default avatar
      else if (currentSrc.includes('avatar') && !currentSrc.includes('avatar.svg')) {
        this.renderer.setAttribute(imgElement, 'src', 'assets/images/avatar.svg');
      }
      // If all else fails, replace with fallback element
      else {
        this.replaceMissingImageWithFallback();
      }
    }
  }
  
  private replaceMissingImageWithFallback() {
    const imgElement = this.el.nativeElement;
    const parent = imgElement.parentNode;
    
    // Do CSS-based DOM fallback replacement
    if (parent && this.fallbackType !== 'default') {
      const fallbackElement = document.createElement('div');
      
      if (this.fallbackType === 'logo') {
        fallbackElement.className = 'habitat-logo-fallback';
        fallbackElement.textContent = 'H';
      } else if (this.fallbackType === 'avatar') {
        fallbackElement.className = 'avatar-fallback';
        fallbackElement.textContent = 'U';
      }
      
      if (fallbackElement.className) {
        this.renderer.setStyle(imgElement, 'display', 'none');
        parent.appendChild(fallbackElement);
      }
    }
  }
}
