import { Injectable, signal } from '@angular/core';
import { TemplateRef } from '@angular/core';

/**
 * Service to manage header content across the application
 * This allows child components rendered in the router-outlet
 * to control header content without direct access to the header component
 */
@Injectable({
  providedIn: 'root'
})
export class HeaderService {
  // Signal for projected title content template
  private _titleTemplateRef = signal<TemplateRef<any> | null>(null);
  
  // Signal for projected actions content template
  private _actionsTemplateRef = signal<TemplateRef<any> | null>(null);
  
  // Signal for header title fallback (used when no template is provided)
  private _titleText = signal<string>('Habitat Builder');
  
  // Getters for templates
  get titleTemplateRef() {
    return this._titleTemplateRef();
  }
  
  get actionsTemplateRef() {
    return this._actionsTemplateRef();
  }
  
  get titleText() {
    return this._titleText();
  }
  
  /**
   * Set the header title template
   * @param templateRef Template reference to project into the header
   */
  setTitleTemplate(templateRef: TemplateRef<any> | null) {
    this._titleTemplateRef.set(templateRef);
  }
  
  /**
   * Set the header actions template
   * @param templateRef Template reference to project into the header
   */
  setActionsTemplate(templateRef: TemplateRef<any> | null) {
    this._actionsTemplateRef.set(templateRef);
  }
  
  /**
   * Set a text title (used as fallback when no template is provided)
   * @param title The title text
   */
  setTitle(title: string) {
    this._titleText.set(title);
  }
  
  /**
   * Clear all header content
   */
  clearAll() {
    this._titleTemplateRef.set(null);
    this._actionsTemplateRef.set(null);
    this._titleText.set('Habitat Builder');
  }
}
