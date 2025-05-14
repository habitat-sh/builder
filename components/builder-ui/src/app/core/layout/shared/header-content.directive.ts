import { Directive, Input, TemplateRef, OnInit, OnDestroy, inject } from '@angular/core';
import { HeaderService } from '../../services/header.service';

/**
 * Directive to set the header title from any component
 * Usage: <ng-template habHeaderTitle>
 *          <h1>My Title</h1>
 *          <h2>Subtitle</h2>
 *        </ng-template>
 */
@Directive({
  selector: '[habHeaderTitle]',
  standalone: true
})
export class HeaderTitleDirective implements OnInit, OnDestroy {
  private headerService = inject(HeaderService);
  private templateRef = inject(TemplateRef<any>);
  
  ngOnInit(): void {
    this.headerService.setTitleTemplate(this.templateRef);
  }
  
  ngOnDestroy(): void {
    // Clear the template reference when the component is destroyed
    this.headerService.setTitleTemplate(null);
  }
}

/**
 * Directive to set the header actions from any component
 * Usage: <ng-template habHeaderActions>
 *          <button mat-button>Action</button>
 *        </ng-template>
 */
@Directive({
  selector: '[habHeaderActions]',
  standalone: true
})
export class HeaderActionsDirective implements OnInit, OnDestroy {
  private headerService = inject(HeaderService);
  private templateRef = inject(TemplateRef<any>);
  
  ngOnInit(): void {
    this.headerService.setActionsTemplate(this.templateRef);
  }
  
  ngOnDestroy(): void {
    // Clear the template reference when the component is destroyed
    this.headerService.setActionsTemplate(null);
  }
}

/**
 * Directive to set a simple text title in the header
 * Usage: <div habHeaderTitleText="My Page Title"></div>
 */
@Directive({
  selector: '[habHeaderTitleText]',
  standalone: true
})
export class HeaderTitleTextDirective implements OnInit, OnDestroy {
  @Input('habHeaderTitleText') title = '';
  
  private headerService = inject(HeaderService);
  
  ngOnInit(): void {
    if (this.title) {
      this.headerService.setTitle(this.title);
    }
  }
  
  ngOnDestroy(): void {
    // Reset the title when the component is destroyed
    this.headerService.setTitle('Habitat Builder');
  }
}
