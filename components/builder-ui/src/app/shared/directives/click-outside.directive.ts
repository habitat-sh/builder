import { Directive, ElementRef, EventEmitter, HostListener, Output } from '@angular/core';

/**
 * A directive that emits an event when a click occurs outside the element it is applied to.
 * Usage: <div (clickOutside)="onClickOutside()">Content</div>
 */
@Directive({
  selector: '[clickOutside]',
  standalone: true
})
export class ClickOutsideDirective {
  @Output() clickOutside = new EventEmitter<void>();

  constructor(private elementRef: ElementRef) {}

  @HostListener('document:click', ['$event.target'])
  onClick(target: HTMLElement): void {
    const clickedInside = this.elementRef.nativeElement.contains(target);
    if (!clickedInside) {
      this.clickOutside.emit();
    }
  }
}
