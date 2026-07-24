import {
  invokeContract,
  waitForConfirmation,
  InvocationConfig,
} from '../invocation';
import { SorobanRpc, Keypair } from '@stellar/stellar-sdk';
import { NetworkError, ContractError } from '../errors';

describe('invocation module', () => {
  let mockServer: any;
  let mockContract: any;
  let mockKeypair: Keypair;
  let config: InvocationConfig;

  beforeEach(() => {
    mockKeypair = Keypair.random();
    mockContract = {
      call: jest.fn(),
    };

    mockServer = {
      simulateTransaction: jest.fn(),
      sendTransaction: jest.fn(),
      getTransaction: jest.fn(),
      getAccount: jest.fn(),
    };

    config = {
      server: mockServer,
      contract: mockContract,
      networkPassphrase: 'Test SDF Network ; September 2015',
      rpcUrl: 'http://localhost:8000',
    };
  });

  describe('waitForConfirmation', () => {
    it('should return immediately on SUCCESS status', async () => {
      const txHash = 'test-hash-123';
      const mockResponse = {
        status: 'SUCCESS',
        resultXdr: 'test-result',
      };

      mockServer.getTransaction.mockResolvedValue(mockResponse);

      const result = await waitForConfirmation(mockServer, txHash);

      expect(result).toEqual(mockResponse);
      expect(mockServer.getTransaction).toHaveBeenCalledWith(txHash);
    });

    it('should throw on FAILED status', async () => {
      const txHash = 'test-hash-123';
      const mockResponse = {
        status: 'FAILED',
        resultXdr: 'failure-xdr',
      };

      mockServer.getTransaction.mockResolvedValue(mockResponse);

      await expect(waitForConfirmation(mockServer, txHash)).rejects.toThrow(
        ContractError
      );
    });

    it('should retry on PENDING status', async () => {
      const txHash = 'test-hash-123';
      const pendingResponse = {
        status: 'PENDING',
      };
      const successResponse = {
        status: 'SUCCESS',
        resultXdr: 'test-result',
      };

      mockServer.getTransaction
        .mockResolvedValueOnce(pendingResponse)
        .mockResolvedValueOnce(pendingResponse)
        .mockResolvedValueOnce(successResponse);

      const result = await waitForConfirmation(mockServer, txHash, 5, 50);

      expect(result).toEqual(successResponse);
      expect(mockServer.getTransaction).toHaveBeenCalledTimes(3);
    });

    it('should throw after max retries on continuous PENDING', async () => {
      const txHash = 'test-hash-123';
      const pendingResponse = {
        status: 'PENDING',
      };

      mockServer.getTransaction.mockResolvedValue(pendingResponse);

      await expect(
        waitForConfirmation(mockServer, txHash, 2, 50)
      ).rejects.toThrow(NetworkError);

      expect(mockServer.getTransaction).toHaveBeenCalledTimes(2);
    });
  });

  describe('invokeContract', () => {
    it('should handle network connection errors', async () => {
      const connectionError = new Error('ECONNREFUSED');
      (connectionError as any).code = 'ECONNREFUSED';

      mockServer.simulateTransaction.mockRejectedValue(connectionError);

      await expect(
        invokeContract('get_balance', [], config, { readOnly: true })
      ).rejects.toThrow(NetworkError);
    });

    it('should handle RPC response errors', async () => {
      const responseError = new Error('RPC Error');
      (responseError as any).response = { status: 500 };

      mockServer.simulateTransaction.mockRejectedValue(responseError);

      await expect(
        invokeContract('get_balance', [], config, { readOnly: true })
      ).rejects.toThrow(NetworkError);
    });

    it('should handle simulation failures', async () => {
      const simulationError = {
        errorMessage: 'Contract invocation failed',
      };

      mockServer.simulateTransaction.mockResolvedValue(simulationError);

      await expect(
        invokeContract('get_balance', [], config, { readOnly: true })
      ).rejects.toThrow(ContractError);
    });
  });

  describe('security', () => {
    it('should not expose keypair in error messages', async () => {
      const connectionError = new Error('Network error');
      (connectionError as any).code = 'ECONNREFUSED';

      mockServer.simulateTransaction.mockRejectedValue(connectionError);

      try {
        await invokeContract('lock_funds', [mockKeypair.publicKey()], config, {
          sourceKeypair: mockKeypair,
        });
      } catch (error: any) {
        // Verify error message doesn't contain secret key
        expect(error.message).not.toContain(mockKeypair.secret());
      }
    });
  });
});
