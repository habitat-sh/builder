import { Directive, Input, ElementRef, OnInit, OnDestroy } from '@angular/core';

/**
 * A directive that automatically focuses an element when it appears in the DOM.
 * Usage: <input autoFocus> or <input [autoFocus]="shouldFocus">
 */
@Directive({
  selector: '[autoFocus]',
  standalone: true
})
export class AutoFocusDirective implements OnInit, OnDestroy {
  @Input() autoFocus = true;
  
  private timeoutId?: number;
  
  constructor(private elementRef: ElementRef) {}
  
  ngOnInit(): void {
    if (this.autoFocus) {
      // Use setTimeout to ensure DOM is fully rendered
      this.timeoutId = window.setTimeout(() => {
        this.elementRef.nativeElement.focus();
      }, 100);
    }
  }
  
  ngOnDestroy(): void {
    if (this.timeoutId) {
      clearTimeout(this.timeoutId);
    }
  }
}
