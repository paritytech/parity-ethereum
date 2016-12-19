// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

// import './integration-tests';
// import './perf-debug';

if (process.env.NODE_ENV === 'development') {
  window.Perf = require('react-addons-perf');
}

if (process.env.NODE_ENV === 'test') {
  require('./tests');
}

const parityDapps = process.env.PARITY_URL ||
  (
    process.env.NODE_ENV === 'production'
      ? '127.0.0.1:8080'
      : ''
  );

const parityUrl = process.env.PARITY_URL ||
  (
    process.env.NODE_ENV === 'production'
      ? window.location.host
      : '127.0.0.1:8180'
  );

export {
  parityDapps,
  parityUrl
};
