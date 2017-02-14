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

export default {
  title: {
    details: `contract details`,
    parameters: `contract parameters`,
    deployment: `deployment`,
    completed: `completed`,
    failed: `deployment failed`,
    rejected: `rejected`
  },
  button: {
    cancel: `Cancel`,
    close: `Close`,
    done: `Done`,
    next: `Next`,
    create: `Create`
  },
  rejected: {
    title: `The deployment has been rejected`,
    description: `You can safely close this window, the contract deployment will not occur.`
  },
  busy: {
    title: `The deployment is currently in progress`
  },
  completed: {
    description: `Your contract has been deployed at`
  },
  owner: {
    noneSelected: `a valid account as the contract owner needs to be selected`
  },
  state: {
    preparing: `Preparing transaction for network transmission`,
    waitSigner: `Waiting for confirmation of the transaction in the Parity Secure Signer`,
    waitReceipt: `Waiting for the contract deployment transaction receipt`,
    validatingCode: `Validating the deployed contract code`,
    completed: `The contract deployment has been completed`
  },
  details: {
    address: {
      hint: `the owner account for this contract`,
      label: `from account (contract owner)`
    },
    name: {
      hint: `a name for the deployed contract`,
      label: `contract name`
    },
    description: {
      hint: `a description for the contract`,
      label: `contract description (optional)`
    },
    abi: {
      hint: `the abi of the contract to deploy or solc combined-output`,
      label: `abi / solc combined-output`
    },
    code: {
      hint: `the compiled code of the contract to deploy`,
      label: `code`
    },
    contract: {
      label: `select a contract`
    }
  },
  parameters: {
    choose: `Choose the contract parameters`
  }
};
