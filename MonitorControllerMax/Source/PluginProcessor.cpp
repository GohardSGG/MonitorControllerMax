/*
  ==============================================================================

    This file contains the basic framework code for a JUCE plugin processor.

  ==============================================================================
*/

#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "InterPluginCommunicator.h"

//==============================================================================
MonitorControllerMaxAudioProcessor::MonitorControllerMaxAudioProcessor()
#ifndef JucePlugin_PreferredChannelConfigurations
     : AudioProcessor (BusesProperties()
                     #if ! JucePlugin_IsMidiEffect
                      #if ! JucePlugin_IsSynth
                       .withInput  ("Input",  juce::AudioChannelSet::create7point1(), true)
                      #endif
                       .withOutput ("Output", juce::AudioChannelSet::create7point1(), true)
                     #endif
                       ),
      apvts (*this, nullptr, "Parameters", createParameterLayout())
#endif
{
    currentRole = standalone;
    communicator = std::make_unique<InterPluginCommunicator>(*this);

    for (int i = 0; i < numManagedChannels; ++i)
    {
        auto muteId = "MUTE_" + juce::String(i + 1);
        auto soloId = "SOLO_" + juce::String(i + 1);
        auto gainId = "GAIN_" + juce::String(i + 1);

        muteParams[i] = apvts.getRawParameterValue(muteId);
        soloParams[i] = apvts.getRawParameterValue(soloId);
        gainParams[i] = apvts.getRawParameterValue(gainId);

        apvts.addParameterListener(muteId, this);
        apvts.addParameterListener(soloId, this);
        apvts.addParameterListener(gainId, this);

        remoteMutes[i] = false;
        remoteSolos[i] = false;
    }
}

MonitorControllerMaxAudioProcessor::~MonitorControllerMaxAudioProcessor()
{
    for (int i = 0; i < numManagedChannels; ++i)
    {
        auto muteId = "MUTE_" + juce::String(i + 1);
        auto soloId = "SOLO_" + juce::String(i + 1);
        auto gainId = "GAIN_" + juce::String(i + 1);
        apvts.removeParameterListener(muteId, this);
        apvts.removeParameterListener(soloId, this);
        apvts.removeParameterListener(gainId, this);
    }
}

//==============================================================================
const juce::String MonitorControllerMaxAudioProcessor::getName() const
{
    return JucePlugin_Name;
}

bool MonitorControllerMaxAudioProcessor::acceptsMidi() const
{
   #if JucePlugin_WantsMidiInput
    return true;
   #else
    return false;
   #endif
}

bool MonitorControllerMaxAudioProcessor::producesMidi() const
{
   #if JucePlugin_ProducesMidiOutput
    return true;
   #else
    return false;
   #endif
}

bool MonitorControllerMaxAudioProcessor::isMidiEffect() const
{
   #if JucePlugin_IsMidiEffect
    return true;
   #else
    return false;
   #endif
}

double MonitorControllerMaxAudioProcessor::getTailLengthSeconds() const
{
    return 0.0;
}

int MonitorControllerMaxAudioProcessor::getNumPrograms()
{
    return 1;   // NB: some hosts don't cope very well if you tell them there are 0 programs,
                // so this should be at least 1, even if you're not really implementing programs.
}

int MonitorControllerMaxAudioProcessor::getCurrentProgram()
{
    return 0;
}

void MonitorControllerMaxAudioProcessor::setCurrentProgram (int index)
{
}

const juce::String MonitorControllerMaxAudioProcessor::getProgramName (int index)
{
    return {};
}

void MonitorControllerMaxAudioProcessor::changeProgramName (int index, const juce::String& newName)
{
}

//==============================================================================
void MonitorControllerMaxAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    // Use this method as the place to do any pre-playback
    // initialisation that you need..
}

void MonitorControllerMaxAudioProcessor::releaseResources()
{
    // When playback stops, you can use this as an opportunity to free up any
    // spare memory, etc.
}

#ifndef JucePlugin_PreferredChannelConfigurations
bool MonitorControllerMaxAudioProcessor::isBusesLayoutSupported (const BusesLayout& layouts) const
{
  #if JucePlugin_IsMidiEffect
    juce::ignoreUnused (layouts);
    return true;
  #else
    // For this validation program, we'll accept any bus layout.
    return true;
  #endif
}
#endif

void MonitorControllerMaxAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;
    auto totalNumInputChannels  = getTotalNumInputChannels();
    auto totalNumOutputChannels = getTotalNumOutputChannels();

    for (auto i = totalNumInputChannels; i < totalNumOutputChannels; ++i)
        buffer.clear (i, 0, buffer.getNumSamples());

    // =================================================================================
    // 1. Determine the final mute/solo state for all managed channels
    // =================================================================================

    bool anySoloEngaged = false;
    const auto role = getRole();

    // First, determine if any solo button is currently active.
    for (int i = 0; i < numManagedChannels; ++i)
    {
        bool isSoloed = (role == Role::slave) ? remoteSolos[i].load()
                                              : soloParams[i]->load() > 0.5f;
        if (isSoloed)
        {
            anySoloEngaged = true;
            break;
        }
    }

    // This array will hold the final decision on whether a channel should be silent.
    std::array<bool, numManagedChannels> channelShouldBeSilent{};

    // Based on the solo state, determine the final mute status for each channel.
    for (int i = 0; i < numManagedChannels; ++i)
    {
        const bool isMuted = (role == Role::slave) ? remoteMutes[i].load()
                                                   : muteParams[i]->load() > 0.5f;

        const bool isSoloed = (role == Role::slave) ? remoteSolos[i].load()
                                                    : soloParams[i]->load() > 0.5f;
        
        // A channel should be silent if it's muted, OR if any solo is engaged and this channel is NOT one of the soloed ones.
        channelShouldBeSilent[i] = isMuted || (anySoloEngaged && !isSoloed);
    }

    // =================================================================================
    // 2. Apply gain and muting to the audio buffer
    // =================================================================================
    for (int channel = 0; channel < totalNumInputChannels; ++channel)
    {
        if (channel >= numManagedChannels)
            continue; // Don't process channels that we are not managing.

        if (channelShouldBeSilent[channel])
        {
            buffer.clear(channel, 0, buffer.getNumSamples());
        }
        else
        {
            // Gain is always sourced from the local APVTS. For slave instances,
            // these values will be at their default (0 dB), so no gain is applied.
            const float gainDb = gainParams[channel]->load();
            if (std::abs(gainDb) > 0.01f) // Small optimization to avoid calculations for 0dB
            {
                buffer.applyGain(channel, 0, buffer.getNumSamples(), juce::Decibels::decibelsToGain(gainDb));
            }
        }
    }
}

//==============================================================================
bool MonitorControllerMaxAudioProcessor::hasEditor() const
{
    return true; // (change this to false if you choose to not supply an editor)
}

juce::AudioProcessorEditor* MonitorControllerMaxAudioProcessor::createEditor()
{
    return new MonitorControllerMaxAudioProcessorEditor (*this);
}

//==============================================================================
void MonitorControllerMaxAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    // You should use this method to store your parameters in the memory block.
    // You could do that either as raw data, or use the XML or ValueTree classes
    // as intermediaries to make it easy to save and load complex data.
}

void MonitorControllerMaxAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    // You should use this method to restore your parameters from this memory block,
    // whose contents will have been created by the getStateInformation() call.
}

juce::AudioProcessorValueTreeState::ParameterLayout MonitorControllerMaxAudioProcessor::createParameterLayout()
{
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;

    for (int i = 0; i < numManagedChannels; ++i)
    {
        juce::String chanNumStr = juce::String(i + 1);

        params.push_back(std::make_unique<juce::AudioParameterBool>("MUTE_" + chanNumStr, "Mute " + chanNumStr, false));
        params.push_back(std::make_unique<juce::AudioParameterBool>("SOLO_" + chanNumStr, "Solo " + chanNumStr, false));
        params.push_back(std::make_unique<juce::AudioParameterFloat>("GAIN_" + chanNumStr, "Gain " + chanNumStr, 
                                                                    juce::NormalisableRange<float>(-100.0f, 12.0f, 0.1f, 3.0f), 0.0f, "dB"));
    }

    return { params.begin(), params.end() };
}

//==============================================================================
// This creates new instances of the plugin..
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new MonitorControllerMaxAudioProcessor();
}

void MonitorControllerMaxAudioProcessor::setRole(Role newRole)
{
    currentRole = newRole;
}

MonitorControllerMaxAudioProcessor::Role MonitorControllerMaxAudioProcessor::getRole() const
{
    return currentRole;
}

void MonitorControllerMaxAudioProcessor::setRemoteMuteSoloState(const MuteSoloState& state)
{
    for (int i = 0; i < numManagedChannels; ++i)
    {
        remoteMutes[i] = state.mutes[i];
        remoteSolos[i] = state.solos[i];
    }
}

bool MonitorControllerMaxAudioProcessor::getRemoteMuteState(int channel) const
{
    if (juce::isPositiveAndBelow(channel, numManagedChannels))
        return remoteMutes[channel].load();
    return false;
}

bool MonitorControllerMaxAudioProcessor::getRemoteSoloState(int channel) const
{
    if (juce::isPositiveAndBelow(channel, numManagedChannels))
        return remoteSolos[channel].load();
    return false;
}

void MonitorControllerMaxAudioProcessor::parameterChanged(const juce::String& parameterID, float newValue)
{
    if (getRole() == Role::master)
    {
        if (parameterID.startsWith("MUTE_") || parameterID.startsWith("SOLO_"))
        {
            MuteSoloState currentState;
            for (int i = 0; i < numManagedChannels; ++i)
            {
                currentState.mutes[i] = muteParams[i]->load() > 0.5f;
                currentState.solos[i] = soloParams[i]->load() > 0.5f;
            }
            communicator->sendMuteSoloState(currentState);
        }
    }
}
