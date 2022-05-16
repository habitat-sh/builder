FROM library/node

WORKDIR /usr/app
COPY package-lock.json /usr/app
COPY package.json /usr/app
ADD src src
ADD fixtures fixtures

RUN npm install -g mocha

RUN npm install chai
RUN npm install supertest
RUN npm install superagent-binary-parser

CMD ["npm", "run", "mocha"]
