Cypress.Commands.add('setSession', () => {
  cy.setCookie('bldrSessionToken', 'CIDAgMSW5MqSEhIGNDc6MTIxGAAiKGRkNGQzNzcwYWM0ZDg5MzNhNjU2NzBkOTQ4MmM1YmMxYTA0Y2ExY2E=');
  cy.setCookie('oauthToken', 'dd4d3770ac4d8933a65670d9482c5ec1a04ca1ca');
  cy.setCookie('oauthState', '612fc2d4-2a28-42be-a3f2-22bd8b5cb1a2');
});

Cypress.Commands.add('clearSession', () => {
  cy.clearCookie('bldrSessionToken');
  cy.clearCookie('oauthToken');
  cy.clearCookie('oauthState');
});
