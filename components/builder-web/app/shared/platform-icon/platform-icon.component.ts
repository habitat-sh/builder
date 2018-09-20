import { Component, Input } from '@angular/core';
import { targetToPlatform } from '../../util';

@Component({
  selector: 'hab-platform-icon',
  template: `<hab-icon [symbol]="os" class="icon-os" [title]="title"></hab-icon>`
})
export class PlatformIconComponent {

  @Input() platform;

  get os() {
    return targetToPlatform(this.platform);
  }

  get title() {
    return {
      linux: 'Linux',
      kernel2: 'Linux (Kernel Version 2)',
      windows: 'Windows'
    }[this.os] || '';
  }
}
