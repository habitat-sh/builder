import { Component } from '@angular/core';

@Component({
  selector: 'app-origins',
  standalone: true,
  template: `
    <div class="placeholder-component">
      <h1>Origins Component</h1>
      <p>This is a placeholder for the Origins feature. It will be implemented in future phases.</p>
    </div>
  `,
  styles: [`
    .placeholder-component {
      padding: 20px;
      text-align: center;
    }
  `]
})
export class OriginsComponent {}
