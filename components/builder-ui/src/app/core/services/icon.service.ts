import { Injectable } from '@angular/core';
import { MatIconRegistry } from '@angular/material/icon';
import { DomSanitizer } from '@angular/platform-browser';

/**
 * Service to register custom SVG icons for use with mat-icon
 */
@Injectable({
  providedIn: 'root'
})
export class IconService {
  constructor(
    private iconRegistry: MatIconRegistry,
    private sanitizer: DomSanitizer
  ) {}

  /**
   * Initialize and register all custom icons
   */
  registerIcons(): void {
    // Register GitHub icon
    this.iconRegistry.addSvgIcon(
      'github',
      this.sanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/github.svg')
    );
    
    // Add other icons as needed
  }
}
