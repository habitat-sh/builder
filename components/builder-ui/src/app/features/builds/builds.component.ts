import { Component } from '@angular/core';

@Component({
  selector: 'app-builds',
  standalone: true,
  template: `
    <div class="placeholder-component">
      <h1>Builds Component</h1>
      <p>This is a placeholder for the Builds feature. It will be implemented in future phases.</p>
    </div>
  `,
  styles: [`
    .placeholder-component {
      padding: 20px;
      text-align: center;
    }
  `]
})
export class BuildsComponent {}
