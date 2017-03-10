// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

import BalancesProvider from './balances';
import { showSnackbar } from './snackbarActions';
import { DEFAULT_NETCHAIN } from './statusReducer';

// Reload the page on network switch after
// 5 seconds
const RELOAD_TIMEOUT = 5000;

export default class ChainMiddleware {
  toMiddleware () {
    return (store) => (next) => (action) => {
      if (action.type === 'statusCollection') {
        const { collection } = action;

        if (collection && collection.netChain) {
          const newChain = collection.netChain;
          const { nodeStatus } = store.getState();

          if (newChain !== nodeStatus.netChain && nodeStatus.netChain !== DEFAULT_NETCHAIN) {
            const timeoutStr = Math.round(RELOAD_TIMEOUT / 1000);

            store.dispatch(showSnackbar(`Switched to ${newChain}. The UI will reload in ${timeoutStr} seconds...`, RELOAD_TIMEOUT));
            setTimeout(() => {
              window.location.reload();
            }, RELOAD_TIMEOUT - 250);

            // Fetch the new balances without notifying the user of any change
            BalancesProvider.get(store).fetchAllBalances({
              changedNetwork: true
            });
          }
        }
      }

      next(action);
    };
  }
}
