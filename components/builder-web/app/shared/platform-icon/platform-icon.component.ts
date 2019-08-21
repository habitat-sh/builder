import { Component, Input } from '@angular/core';
import { targetFrom } from '../../util';

@Component({
  selector: 'hab-platform-icon',
  template: `<hab-icon *ngIf="target" [symbol]="target.param" class="icon-os" [title]="target.title"></hab-icon>`
})
export class PlatformIconComponent {

  @Input() platform;

  get target() {
    return targetFrom('id', this.platform);
  }
}
