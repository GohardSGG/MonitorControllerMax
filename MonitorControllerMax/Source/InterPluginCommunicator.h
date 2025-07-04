/*
  ==============================================================================

    InterPluginCommunicator.h
    Created: 4 Aug 2024 10:00:00am
    Author:  Your Name

  ==============================================================================
*/

#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"

//==============================================================================
/*
*/
class InterPluginCommunicator   : public juce::InterprocessConnectionServer,
                                  public juce::InterprocessConnection
{
public:
    class ClientConnection : public juce::InterprocessConnection
    {
    public:
        ClientConnection(InterPluginCommunicator& owner) : InterprocessConnection(false, 0), ownerCommunicator(owner) {}
        ~ClientConnection() override {}

        void connectionMade() override { ownerCommunicator.clientConnected(this); }
        void connectionLost() override { ownerCommunicator.clientDisconnected(this); }
        void messageReceived(const juce::MemoryBlock& message) override {}

    private:
        InterPluginCommunicator& ownerCommunicator;
    };

    InterPluginCommunicator(MonitorControllerMaxAudioProcessor& p);
    ~InterPluginCommunicator() override;

    void sendMuteSoloState(const MonitorControllerMaxAudioProcessor::MuteSoloState& state);
    void clientConnected(ClientConnection* client);
    void clientDisconnected(ClientConnection* client);

    // juce::InterprocessConnectionServer overrides
    juce::InterprocessConnection* createConnectionObject() override;

    // juce::InterprocessConnection overrides
    void connectionMade() override;
    void connectionLost() override;
    void messageReceived(const juce::MemoryBlock& message) override;

private:
    void setRole(MonitorControllerMaxAudioProcessor::Role newRole);

    MonitorControllerMaxAudioProcessor& processor;
    std::vector<ClientConnection*> connectedClients;

    static const int ipcPort = 9001; // An arbitrary port number for our IPC
    static const int reconnectIntervalMs = 5000; // How often a slave tries to reconnect

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (InterPluginCommunicator)
}; 