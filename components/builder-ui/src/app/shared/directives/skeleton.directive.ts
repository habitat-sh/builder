import { Directive, ElementRef, Input, OnInit, OnChanges, SimpleChanges } from '@angular/core';

/**
 * A directive that creates a skeleton loading effect for content that is loading.
 * Usage: <div [skeleton]="isLoading" [skeletonWidth]="'100%'" [skeletonHeight]="'20px'">Content</div>
 */
@Directive({
  selector: '[skeleton]',
  standalone: true
})
export class SkeletonDirective implements OnInit, OnChanges {
  @Input() skeleton = false;
  @Input() skeletonWidth = '100%';
  @Input() skeletonHeight = '16px';
  @Input() skeletonBorderRadius = '4px';
  
  private originalStyles: {
    backgroundColor?: string;
    color?: string;
    borderColor?: string;
    backgroundImage?: string;
    width?: string;
    height?: string;
    position?: string;
    overflow?: string;
  } = {};
  
  private originalInnerHTML = '';
  
  constructor(private el: ElementRef) {}
  
  ngOnInit(): void {
    this.saveOriginalStyles();
    this.applyStyles();
  }
  
  ngOnChanges(changes: SimpleChanges): void {
    if (changes['skeleton']) {
      this.applyStyles();
    }
  }
  
  private saveOriginalStyles(): void {
    const element = this.el.nativeElement;
    const styles = window.getComputedStyle(element);
    
    this.originalStyles = {
      backgroundColor: element.style.backgroundColor,
      color: element.style.color,
      borderColor: element.style.borderColor,
      backgroundImage: element.style.backgroundImage,
      width: element.style.width,
      height: element.style.height,
      position: element.style.position,
      overflow: element.style.overflow
    };
    
    this.originalInnerHTML = element.innerHTML;
  }
  
  private applyStyles(): void {
    const element = this.el.nativeElement;
    
    if (this.skeleton) {
      // Apply skeleton styles
      element.style.backgroundColor = '#e0e0e0';
      element.style.color = 'transparent';
      element.style.borderColor = 'transparent';
      element.style.backgroundImage = 'linear-gradient(90deg, rgba(255, 255, 255, 0), rgba(255, 255, 255, 0.5), rgba(255, 255, 255, 0))';
      element.style.backgroundSize = '200px 100%';
      element.style.backgroundRepeat = 'no-repeat';
      element.style.backgroundPosition = 'left -150px top 0';
      element.style.animation = 'skeleton-loading 1.5s ease-in-out infinite';
      element.style.borderRadius = this.skeletonBorderRadius;
      element.style.width = this.skeletonWidth;
      element.style.height = this.skeletonHeight;
      element.style.position = 'relative';
      element.style.overflow = 'hidden';
      element.innerHTML = '';
      
      // Add animation style if not already present
      this.addAnimationStyle();
      
    } else {
      // Restore original styles
      Object.keys(this.originalStyles).forEach(key => {
        const value = this.originalStyles[key as keyof typeof this.originalStyles];
        if (value) {
          element.style[key as any] = value;
        } else {
          element.style.removeProperty(key);
        }
      });
      
      element.style.removeProperty('background-size');
      element.style.removeProperty('background-repeat');
      element.style.removeProperty('background-position');
      element.style.removeProperty('animation');
      element.style.removeProperty('border-radius');
      
      // Restore original content
      element.innerHTML = this.originalInnerHTML;
    }
  }
  
  private addAnimationStyle(): void {
    const id = 'skeleton-animation-style';
    if (!document.getElementById(id)) {
      const style = document.createElement('style');
      style.id = id;
      style.innerHTML = `
        @keyframes skeleton-loading {
          0% {
            background-position: left -150px top 0;
          }
          100% {
            background-position: right -150px top 0;
          }
        }
      `;
      document.head.appendChild(style);
    }
  }
}
