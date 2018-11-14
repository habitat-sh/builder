import { Component, Input } from '@angular/core';

@Component({
  selector: 'hab-visibility-icon',
  template: `<hab-icon [symbol]="symbol" class="icon-visibility" [title]="title"></hab-icon>`
})
export class VisibilityIconComponent {

  @Input() visibility: string;
  @Input() prefix: string;

  get symbol() {
    return this.visibility === 'public' ? 'public' : 'lock';
  }

  get title() {
    const t = this.visibility === 'public' ? 'Public' : 'Private';
    return this.prefix ? `${this.prefix} ${t}` : t;
  }
}
