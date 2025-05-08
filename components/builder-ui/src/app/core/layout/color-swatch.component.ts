import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-color-swatch',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="container">
      <h1>Habitat Builder UI Color Palette</h1>
      <p>These are the standardized colors used throughout the application.</p>
      
      <h2>Primary Colors</h2>
      <div class="color-grid">
        <div class="color-swatch dark-blue">
          <div class="swatch-name">Dark Blue</div>
          <div class="swatch-hex">#283C4C</div>
        </div>
        <div class="color-swatch medium-blue">
          <div class="swatch-name">Medium Blue</div>
          <div class="swatch-hex">#556F84</div>
        </div>
        <div class="color-swatch hab-blue">
          <div class="swatch-name">Habitat Blue</div>
          <div class="swatch-hex">#4296b4</div>
        </div>
        <div class="color-swatch hab-green">
          <div class="swatch-name">Habitat Green</div>
          <div class="swatch-hex">#87B09A</div>
        </div>
        <div class="color-swatch hab-orange">
          <div class="swatch-name">Habitat Orange</div>
          <div class="swatch-hex">#FF9012</div>
        </div>
        <div class="color-swatch hab-red">
          <div class="swatch-name">Habitat Red</div>
          <div class="swatch-hex">#EB6852</div>
        </div>
      </div>
      
      <h2>Status Colors</h2>
      <div class="color-grid">
        <div class="color-swatch active">
          <div class="swatch-name">Active</div>
        </div>
        <div class="color-swatch success">
          <div class="swatch-name">Success</div>
        </div>
        <div class="color-swatch pending">
          <div class="swatch-name">Pending</div>
        </div>
        <div class="color-swatch waiting">
          <div class="swatch-name">Waiting</div>
        </div>
        <div class="color-swatch warn">
          <div class="swatch-name">Warning</div>
        </div>
        <div class="color-swatch error">
          <div class="swatch-name">Error</div>
        </div>
      </div>
    </div>
  `,
  styles: [`
    @use '../../core/styles/colors' as *;
    @use '../../core/styles/typography' as *;
    @use '../../core/styles/mixins' as mix;
    
    .container {
      padding: 32px;
    }
    
    h1, h2 {
      color: $dark-blue;
    }
    
    .color-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
      gap: 16px;
      margin: 24px 0;
    }
    
    .color-swatch {
      height: 100px;
      border-radius: 4px;
      display: flex;
      flex-direction: column;
      justify-content: flex-end;
      padding: 12px;
      color: white;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
      
      .swatch-name {
        font-weight: 600;
      }
      
      .swatch-hex {
        font-family: $monospace-font-family;
        font-size: 12px;
        opacity: 0.8;
      }
    }
    
    .dark-blue { background-color: $dark-blue; }
    .medium-blue { background-color: $medium-blue; }
    .hab-blue { background-color: $hab-blue; }
    .hab-green { background-color: $hab-green; }
    .hab-orange { background-color: $hab-orange; }
    .hab-red { background-color: $hab-red; }
    
    .active { background-color: $active; }
    .success { background-color: $success; }
    .pending { background-color: $pending; }
    .waiting { background-color: $waiting; }
    .warn { background-color: $warn; }
    .error { background-color: $error; }
  `]
})
export class ColorSwatchComponent {}
