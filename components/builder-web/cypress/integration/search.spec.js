describe('Search', () => {

  beforeEach(() => {
    cy.server();
  });

  describe('when the API returns one or more packages', () => {

    beforeEach(() => {
      cy.route({
        method: 'GET',
        url: '/v1/depot/pkgs/core?range=0&distinct=true',
        response: {
          range_start: 0,
          range_end: 1,
          total_count: 2,
          data: [
            {name: "gawk", origin: "core"},
            {name: "cacerts", origin: "core"}
          ]
        }
      });

      cy.visit('/#/pkgs/core');
    });

    it('lists them', () => {
      cy.get('.results-component li').children().should('have.length', 2);
    });
  });

  describe('when the API returns no packages', () => {

    beforeEach(() => {
      cy.route({
        method: 'GET',
        url: '/v1/depot/pkgs/core?range=0&distinct=true',
        response: {
          range_start: 0,
          range_end: 1,
          total_count: 0,
          data: []
        }
      });

      cy.visit('/#/pkgs/core');
    });

    it('indicates that none were found', () => {
      cy.get('.results-component .none').should('contain', 'No packages found.');
    });
  })
});
